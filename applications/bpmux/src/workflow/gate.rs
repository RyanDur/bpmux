use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;

use super::config::{GateClass, WorkflowConfig};

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GateError {
    #[error(
        "workflow-transition-undefined: '{attempted_transition}' is not declared in workflow.toml"
    )]
    #[serde(rename = "workflow-transition-undefined")]
    TransitionUndefined { attempted_transition: String },
    #[error(
        "workflow-role-not-allowed: role '{actor_role}' cannot fire '{attempted_transition}' — allowed roles: {allowed_roles:?}"
    )]
    #[serde(rename = "workflow-role-not-allowed")]
    RoleNotAllowed {
        actor_role: String,
        attempted_transition: String,
        allowed_roles: Vec<String>,
    },
    #[error(
        "workflow-gate-mismatch: '{attempted_transition}' for class {class:?} missing evidence {missing_evidence:?}"
    )]
    #[serde(rename = "workflow-gate-mismatch")]
    GateMismatch {
        actor_role: String,
        attempted_transition: String,
        class: GateClass,
        missing_evidence: Vec<String>,
    },
}

/// A minimal slice of a channel message — only the fields the gate
/// validator needs to attribute a verdict to a role within a window.
#[derive(Debug, Clone)]
pub struct EvidenceMessage {
    pub from_role: String,
    pub ts: DateTime<Utc>,
}

pub fn check_evidence_present(
    workflow: &WorkflowConfig,
    transition_id: &str,
    actor_role: &str,
    messages: &[EvidenceMessage],
    now: DateTime<Utc>,
) -> Result<(), GateError> {
    let Some(transition) = workflow.transition_by_id(transition_id) else {
        return Err(GateError::TransitionUndefined {
            attempted_transition: transition_id.to_string(),
        });
    };
    if transition.gate_class == GateClass::None || transition.required_evidence_roles.is_empty() {
        return Ok(());
    }
    let window = chrono::Duration::seconds(transition.evidence_window_seconds as i64);
    let cutoff = now - window;
    let mut missing: Vec<String> = Vec::new();
    for required in &transition.required_evidence_roles {
        let has = messages
            .iter()
            .any(|m| m.from_role == *required && m.ts >= cutoff);
        if !has {
            missing.push(format!("{required}-verdict"));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(GateError::GateMismatch {
            actor_role: actor_role.to_string(),
            attempted_transition: transition_id.to_string(),
            class: transition.gate_class,
            missing_evidence: missing,
        })
    }
}

pub fn check_role_allowed(
    workflow: &WorkflowConfig,
    transition_id: &str,
    actor_role: &str,
) -> Result<(), GateError> {
    let Some(transition) = workflow.transition_by_id(transition_id) else {
        return Err(GateError::TransitionUndefined {
            attempted_transition: transition_id.to_string(),
        });
    };
    if transition.actor_roles.iter().any(|r| r == actor_role) {
        Ok(())
    } else {
        Err(GateError::RoleNotAllowed {
            actor_role: actor_role.to_string(),
            attempted_transition: transition_id.to_string(),
            allowed_roles: transition.actor_roles.clone(),
        })
    }
}

impl GateError {
    pub fn as_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.to_string())
    }
}
