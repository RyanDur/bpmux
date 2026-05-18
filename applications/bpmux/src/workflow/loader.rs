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

/// The bpmux-canonical 3-section skeleton. workflow.toml's
/// `[[state]]` rows must use one of these section names; any other
/// value is a canon-violation (#1394 substrate-enforce).
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
    let content = std::fs::read_to_string(&path).map_err(|e| WorkflowLoadError::ReadFailed {
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

/// #1394 substrate-enforce: every state must declare one of the
/// CANONICAL_SECTIONS, and each canonical section must have at least
/// one state. Empty-states is semantically malformed.
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
