use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

use super::config::{GateClass, Transition, WorkflowConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictState {
    pub cast_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub stale_age_secs: Option<i64>,
}

pub fn verdict_freshness(
    cast_at: Option<DateTime<Utc>>,
    window_secs: u64,
    now: DateTime<Utc>,
) -> VerdictState {
    let window = chrono::Duration::seconds(window_secs as i64);
    let expired_at = cast_at.map(|ts| ts + window);
    let stale_age_secs = expired_at
        .filter(|exp| *exp < now)
        .map(|exp| (now - exp).num_seconds());
    VerdictState {
        cast_at,
        expired_at,
        stale_age_secs,
    }
}

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

#[derive(Debug, Clone)]
pub struct CiSnapshot {
    pub conclusion: String,
    pub head_sha: String,
}

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
    let mut verdict_state: BTreeMap<String, VerdictState> = BTreeMap::new();
    for required in &transition.required_evidence_roles {
        let latest_for_role = messages
            .iter()
            .filter(|m| m.from_role == *required)
            .max_by_key(|m| m.ts)
            .map(|m| m.ts);
        let has_in_window = latest_for_role.is_some_and(|ts| ts >= cutoff);
        if !has_in_window {
            match latest_for_role {
                None => missing.push(format!("{required}-verdict-missing")),
                Some(_) => missing.push(format!("{required}-verdict-expired")),
            }
        }
        verdict_state.insert(
            required.clone(),
            verdict_freshness(latest_for_role, transition.evidence_window_seconds, now),
        );
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(GateError::GateMismatch {
            actor_role: actor_role.to_string(),
            attempted_transition: transition.id.clone(),
            class: transition.gate_class,
            missing_evidence: missing,
            verdict_state: Some(verdict_state),
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
