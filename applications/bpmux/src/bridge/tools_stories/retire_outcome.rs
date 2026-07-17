use super::event_meta::meta_for;
use super::helpers::{ensure_outcome_in_canonical_store, require_purpose};
use crate::sync::SyncOrchestrator;
use product_management::story::StoryId;
use product_management::story::events::{StoryEvent, StoryEventKind};
use std::sync::Arc;

pub(crate) async fn handle_retire_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "retire an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let reason = args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .ok_or("outcome retire needs --reason <R> (retire is destructive; the reason is the audit-trail)")?;

    let driver = refresh.ok_or("retire_outcome requires refresh")?;
    let outcome = ensure_outcome_in_canonical_store(driver, select, outcome_id).await?;

    let meta = || meta_for(agent_name, actor_id);
    driver
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(outcome_id),
            StoryEventKind::StateTransitioned {
                new_state: product_management::story::Work::Retired,
                placement: None,
            },
        ))
        .map_err(|e| format!("#{outcome_id} retire state-transition failed: {e}"))?;
    driver
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(outcome_id),
            StoryEventKind::StoryClosed {
                reason: Some(format!("retired: {reason}")),
            },
        ))
        .map_err(|e| format!("#{outcome_id} retire enqueue failed: {e}"))?;
    if !outcome.is_retired() {
        driver
            .mutate(StoryEvent::new(
                meta(),
                StoryId::new(outcome_id),
                StoryEventKind::TagAdded {
                    label: "retired".to_string(),
                },
            ))
            .map_err(|e| format!("#{outcome_id} retire-tag enqueue failed: {e}"))?;
    }

    dispatch.dispatch(crate::store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![outcome.retired()]),
    ));

    let orphaned = outcome.iterations.len();
    let mapping_note = match orphaned {
        0 => String::new(),
        n => format!(" — {n} iteration(s) now unmapped (re-home: outcome add ...)"),
    };
    Ok(format!(
        "#{outcome_id} {} retired (reason: {reason}){mapping_note}; it no longer appears in bpmux roadmap",
        outcome.title
    ))
}
