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
    WORKFLOW.get_or_init(|| match load_workflow_from_process_env() {
        Ok(config) => config,
        Err(e) => {
            crate::diag::log_diag(&format!(
                "workflow config load failed at lazy init; falling back to bundled default: {e}"
            ));
            bundled_default()
        }
    })
}

pub fn load_workflow_from_process_env() -> Result<WorkflowConfig, WorkflowLoadError> {
    let override_path = std::env::var_os("BPMUX_WORKFLOW_CONFIG").map(PathBuf::from);
    let project_root = std::env::var_os("BPMUX_PROJECT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(load_project_root_or_cwd);
    load_workflow_with_override(override_path.as_deref(), &project_root)
}

pub fn load_workflow_with_override(
    override_path: Option<&Path>,
    project_root: &Path,
) -> Result<WorkflowConfig, WorkflowLoadError> {
    match override_path {
        Some(path) => load_workflow_file(path),
        None => load_workflow(project_root),
    }
}

pub fn load_workflow_file(path: &Path) -> Result<WorkflowConfig, WorkflowLoadError> {
    let content = std::fs::read_to_string(path).map_err(|e| WorkflowLoadError::ReadFailed {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let config: WorkflowConfig =
        toml::from_str(&content).map_err(|e| WorkflowLoadError::ParseFailed {
            path: path.display().to_string(),
            parse_error: e.to_string(),
        })?;
    validate_section_skeleton(&config)?;
    Ok(config)
}

pub const CANONICAL_SECTIONS: &[&str] = &["pre-work", "in-work", "post-work"];

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WorkflowLoadError {
    #[error("workflow-config-invalid: failed to read {path}: {message}")]
    #[serde(rename = "workflow-config-read-failed")]
    ReadFailed { path: String, message: String },
    #[error("workflow-config-invalid: TOML parse error in {path}: {parse_error}")]
    #[serde(rename = "workflow-config-invalid")]
    ParseFailed { path: String, parse_error: String },
    #[error(
        "workflow-canon-violation: state '{state_id}' declares section '{invalid_section}'; valid sections are {valid_sections:?}"
    )]
    #[serde(rename = "workflow-canon-violation")]
    CanonViolation {
        invalid_section: String,
        valid_sections: Vec<String>,
        state_id: String,
    },
    #[error(
        "workflow-empty-states: workflow.toml requires at least one [[state]] per canonical section; missing: {missing_sections:?}"
    )]
    #[serde(rename = "workflow-empty-states")]
    EmptyStates { missing_sections: Vec<String> },
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
    load_workflow_file(&path)
}

pub fn validate_section_skeleton(config: &WorkflowConfig) -> Result<(), WorkflowLoadError> {
    for state in &config.state {
        if !CANONICAL_SECTIONS.iter().any(|c| *c == state.section) {
            return Err(WorkflowLoadError::CanonViolation {
                invalid_section: state.section.clone(),
                valid_sections: CANONICAL_SECTIONS.iter().map(|s| s.to_string()).collect(),
                state_id: state.id.clone(),
            });
        }
    }
    let missing: Vec<String> = CANONICAL_SECTIONS
        .iter()
        .filter(|c| !config.state.iter().any(|s| s.section == **c))
        .map(|c| c.to_string())
        .collect();
    if !missing.is_empty() {
        return Err(WorkflowLoadError::EmptyStates {
            missing_sections: missing,
        });
    }
    Ok(())
}
