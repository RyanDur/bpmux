use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;

use super::super::config::{GateClass, Transition};
use super::GateError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictState {
    pub cast_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub stale_age_secs: Option<i64>,
    pub substrate_changed: bool,
}

impl VerdictState {
    pub fn is_fresh(&self) -> bool {
        self.cast_at.is_some() && self.stale_age_secs.is_none() && !self.substrate_changed
    }
}

pub fn verdict_freshness(
    cast_at: Option<DateTime<Utc>>,
    window_secs: u64,
    now: DateTime<Utc>,
    grounded_head: Option<&str>,
    current_head: Option<&str>,
) -> VerdictState {
    let window = chrono::Duration::seconds(window_secs as i64);
    let expired_at = cast_at.map(|ts| ts + window);
    let stale_age_secs = expired_at
        .filter(|exp| *exp < now)
        .map(|exp| (now - exp).num_seconds());
    let substrate_changed = match (grounded_head, current_head) {
        (Some(grounded), Some(current)) => grounded != current,
        _ => false,
    };
    VerdictState {
        cast_at,
        expired_at,
        stale_age_secs,
        substrate_changed,
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceMessage {
    pub from_role: String,
    pub ts: DateTime<Utc>,
    pub grounded_head: Option<String>,
}

pub fn check_evidence_present(
    transition: &Transition,
    actor_role: &str,
    messages: &[EvidenceMessage],
    now: DateTime<Utc>,
    current_head: Option<&str>,
) -> Result<(), GateError> {
    if transition.gate_class == GateClass::None || transition.required_evidence_roles.is_empty() {
        return Ok(());
    }
    let mut missing: Vec<String> = Vec::new();
    let mut verdict_state: BTreeMap<String, VerdictState> = BTreeMap::new();
    for required in &transition.required_evidence_roles {
        let latest = messages
            .iter()
            .filter(|m| m.from_role == *required)
            .max_by_key(|m| m.ts);
        let latest_for_role = latest.map(|m| m.ts);
        let grounded_head = latest.and_then(|m| m.grounded_head.as_deref());
        let state = verdict_freshness(
            latest_for_role,
            transition.evidence_window_seconds,
            now,
            grounded_head,
            current_head,
        );
        if latest_for_role.is_none() {
            missing.push(format!("{required}-verdict-missing"));
        } else if !state.is_fresh() {
            missing.push(format!("{required}-verdict-expired"));
        }
        verdict_state.insert(required.clone(), state);
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
