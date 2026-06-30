use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId};
use crate::product_management::story::StoryId;
use std::sync::Arc;

pub(crate) async fn handle_retire_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "retire an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let reason = args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .ok_or("outcome retire needs --reason <R> (retire is destructive; the reason is the audit-trail)")?;

    let r = refresh.ok_or("retire_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_outcome_in_canonical_store(r, outcome_id).await?;

    let meta = || EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };
    r.mutate_sync(BoardEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(outcome_id),
        new_state: crate::product_management::story::StoryState::Retired,
    })
    .map_err(|e| format!("#{outcome_id} retire state-transition failed: {e}"))?;
    r.mutate_sync(BoardEvent::StoryClosed {
        meta: meta(),
        id: StoryId::new(outcome_id),
        reason: Some(format!("retired: {reason}")),
    })
    .map_err(|e| format!("#{outcome_id} retire enqueue failed: {e}"))?;
    if !outcome.is_retired() {
        r.mutate_sync(BoardEvent::TagAdded {
            meta: meta(),
            id: StoryId::new(outcome_id),
            label: "retired".to_string(),
        })
        .map_err(|e| format!("#{outcome_id} retire-tag enqueue failed: {e}"))?;
    }

    let orphaned = outcome.iterations.len();
    let mapping_note = match orphaned {
        0 => String::new(),
        n => format!(" — {n} iteration(s) now unmapped (re-home: outcome add ...)"),
    };
    Ok(format!(
        "#{outcome_id} {} retired (reason: {reason}){mapping_note}; it no longer appears in bpmux roadmap",
        outcome.content.title
    ))
}
