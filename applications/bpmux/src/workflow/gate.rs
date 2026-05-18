use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;

use super::config::{GateClass, Transition, WorkflowConfig};

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

/// CI-state snapshot the workflow gate consumes — abstracts the
/// `get_ci_state` shape down to the fields the validator needs so
/// `check_ci_green` stays domain-independent. Caller maps the bridge's
/// real CI state into this struct.
#[derive(Debug, Clone)]
pub struct CiSnapshot {
    pub conclusion: String,
    pub head_sha: String,
}

/// #1420 dual-gate sister to evidence + role checks: refuse the
/// transition fire when CI is not green on the work's head_sha.
/// Per Ryan-canon 2026-05-18 17:25/17:27 UTC: pass_desk_check
/// (and sister CI-gated transitions) require either CI green on
/// the work's commit OR the work's commit in the ancestry of a
/// green CI head. This MVP slice checks the head_sha (single point);
/// proper git-ancestry-traversal is a sister-touch when surfacing.
///
/// quota_exhausted is treated as an exception class — the gate
/// returns Ok so the caller can rely on a separate evidence-cite
/// (matching pre-accept-ci-check skill's quota-exception canon).
pub fn check_ci_green(
    transition: &Transition,
    snapshot: Option<&CiSnapshot>,
) -> Result<(), GateError> {
    let Some(snap) = snapshot else {
        return Err(GateError::CiNotGreen {
            attempted_transition: transition.id.clone(),
            ci_conclusion: "unknown".to_string(),
            head_sha: String::new(),
            green_head_check_path: "neither".to_string(),
        });
    };
    match snap.conclusion.as_str() {
        "success" => Ok(()),
        "quota_exhausted" => Ok(()),
        other => Err(GateError::CiNotGreen {
            attempted_transition: transition.id.clone(),
            ci_conclusion: other.to_string(),
            head_sha: snap.head_sha.clone(),
            green_head_check_path: "neither".to_string(),
        }),
    }
}

/// A minimal slice of a channel message — only the fields the gate
/// validator needs to attribute a verdict to a role within a window.
#[derive(Debug, Clone)]
pub struct EvidenceMessage {
    pub from_role: String,
    pub ts: DateTime<Utc>,
}

pub fn check_evidence_present(
    transition: &Transition,
    actor_role: &str,
    messages: &[EvidenceMessage],
    now: DateTime<Utc>,
) -> Result<(), GateError> {
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
            attempted_transition: transition.id.clone(),
            class: transition.gate_class,
            missing_evidence: missing,
        })
    }
}

pub fn check_role_allowed(transition: &Transition, actor_role: &str) -> Result<(), GateError> {
    if transition.actor_roles.iter().any(|r| r == actor_role) {
        Ok(())
    } else {
        Err(GateError::RoleNotAllowed {
            actor_role: actor_role.to_string(),
            attempted_transition: transition.id.clone(),
            allowed_roles: transition.actor_roles.clone(),
        })
    }
}

/// Refuse a transition fire when the story is already in the
/// transition's target state — covers the concurrent-fire race where
/// a second fire arrives after the first has mutated state to the
/// target. The error cites the from-state as `expected_state` so the
/// caller can see what state the transition expected.
pub fn check_target_state_valid(
    transition: &Transition,
    current_state: &str,
) -> Result<(), GateError> {
    if current_state == transition.to {
        return Err(GateError::TargetStateInvalid {
            attempted_transition: transition.id.clone(),
            current_state: current_state.to_string(),
            expected_state: transition.from.clone(),
        });
    }
    Ok(())
}

/// Lookup helper that returns the structured TransitionUndefined
/// error when no matching transition exists for (id, class).
pub fn resolve_transition<'a>(
    workflow: &'a WorkflowConfig,
    transition_id: &str,
    class: GateClass,
) -> Result<&'a Transition, GateError> {
    workflow
        .transition_for(transition_id, class)
        .ok_or_else(|| GateError::TransitionUndefined {
            attempted_transition: transition_id.to_string(),
        })
}

/// One row of the story's prior gate-fire history — the slice of
/// `GateComment` the self-mutation guard needs to attribute prior
/// fires to actors. Caller maps `board::domain::GateComment` →
/// PriorFire so the validator stays domain-independent.
#[derive(Debug, Clone)]
pub struct PriorFire {
    pub gate_id: String,
    pub actor_role: String,
}

pub fn check_self_mutation_refused(
    actor_role: &str,
    attempted_transition: &str,
    prior_fires: &[PriorFire],
    refused_prior_gates: &[&str],
) -> Result<(), GateError> {
    for prior in prior_fires {
        if refused_prior_gates.contains(&prior.gate_id.as_str()) && prior.actor_role == actor_role {
            return Err(GateError::SelfMutationRefused {
                actor_role: actor_role.to_string(),
                attempted_transition: attempted_transition.to_string(),
                prior_fire: prior.gate_id.clone(),
            });
        }
    }
    Ok(())
}

impl GateError {
    pub fn as_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.to_string())
    }
}
