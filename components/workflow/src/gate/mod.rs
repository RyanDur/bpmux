use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

use super::config::GateClass;

mod ci;
mod freshness;
mod role;
mod transition;

pub use ci::{CiSnapshot, check_ci_green};
pub use freshness::{EvidenceMessage, VerdictState, check_evidence_present, verdict_freshness};
pub use role::{check_role_allowed, enforce_role_allowed, enforce_stage_judge};
pub use transition::{
    PriorFire, check_self_mutation_refused, check_target_state_valid, resolve_transition,
};

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
        #[serde(skip_serializing_if = "Option::is_none")]
        verdict_state: Option<BTreeMap<String, VerdictState>>,
    },
    #[error(
        "workflow-target-state-invalid: story is {current_state}, expected {expected_state} for '{attempted_transition}'"
    )]
    #[serde(rename = "workflow-target-state-invalid")]
    TargetStateInvalid {
        attempted_transition: String,
        current_state: String,
        expected_state: String,
    },
    #[error(
        "workflow-self-mutation-refused: actor '{actor_role}' previously fired '{prior_fire}'; cannot fire '{attempted_transition}'"
    )]
    #[serde(rename = "workflow-self-mutation-refused")]
    SelfMutationRefused {
        actor_role: String,
        attempted_transition: String,
        prior_fire: String,
    },
    #[error(
        "workflow-ci-not-green: '{attempted_transition}' refused — CI conclusion '{ci_conclusion}' on head '{head_sha}' (path: {green_head_check_path})"
    )]
    #[serde(rename = "workflow-ci-not-green")]
    CiNotGreen {
        attempted_transition: String,
        ci_conclusion: String,
        head_sha: String,
        green_head_check_path: String,
    },
}

impl GateError {
    pub fn as_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.to_string())
    }

    pub fn expired_evidence_roles(&self) -> Vec<String> {
        const SUFFIX: &str = "-verdict-expired";
        match self {
            GateError::GateMismatch {
                missing_evidence, ..
            } => missing_evidence
                .iter()
                .filter_map(|m| m.strip_suffix(SUFFIX))
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        }
    }
}
