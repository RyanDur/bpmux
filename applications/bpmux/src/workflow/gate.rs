use serde::Serialize;
use thiserror::Error;

use super::config::WorkflowConfig;

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
