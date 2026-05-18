use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use thiserror::Error;

use super::bundled_default::bundled_default;
use super::config::WorkflowConfig;

static WORKFLOW: OnceLock<WorkflowConfig> = OnceLock::new();

pub fn install_workflow(config: WorkflowConfig) {
    let _ = WORKFLOW.set(config);
}

pub fn current_workflow() -> &'static WorkflowConfig {
    WORKFLOW.get_or_init(bundled_default)
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WorkflowLoadError {
    #[error("workflow-config-invalid: failed to read {path}: {message}")]
    #[serde(rename = "workflow-config-read-failed")]
    ReadFailed { path: String, message: String },
    #[error("workflow-config-invalid: TOML parse error in {path}: {parse_error}")]
    #[serde(rename = "workflow-config-invalid")]
    ParseFailed { path: String, parse_error: String },
}

pub fn workflow_path(project_root: &Path) -> PathBuf {
    project_root.join(".bpmux").join("workflow.toml")
}

pub fn load_project_root_or_cwd() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if ancestor.join(".git").exists() {
                return ancestor.to_path_buf();
            }
        }
        return cwd;
    }
    PathBuf::from(".")
}

pub fn load_workflow(project_root: &Path) -> Result<WorkflowConfig, WorkflowLoadError> {
    let path = workflow_path(project_root);
    if !path.exists() {
        return Ok(bundled_default());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| WorkflowLoadError::ReadFailed {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    toml::from_str::<WorkflowConfig>(&content).map_err(|e| WorkflowLoadError::ParseFailed {
        path: path.display().to_string(),
        parse_error: e.to_string(),
    })
}
