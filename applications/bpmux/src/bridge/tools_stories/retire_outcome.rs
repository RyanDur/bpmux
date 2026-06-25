use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId};
use crate::product_management::story::StoryType;
use std::sync::Arc;

pub(crate) async fn handle_retire_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "retire an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
    let reason = args
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .ok_or("outcome retire needs --reason <R> (retire is destructive; the reason is the audit-trail)")?;

    let r = refresh.ok_or("retire_outcome requires refresh (RefreshDriver)")?;
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    let story_type =
        crate::product_management::board::adapters::github::story_type_from_labels_or_title(
            &outcome.raw_labels,
            &outcome.title,
        );
    if story_type != StoryType::Outcome {
        return Err(format!(
            "#{outcome_number} is not an Outcome (story_type={:?}); it may be a story, not an outcome — outcomes: bpmux roadmap",
            story_type
        ));
    }

    let meta = || EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };
    r.mutate_sync(BoardEvent::StateTransitioned {
        meta: meta(),
        number: outcome_number,
        new_state: crate::product_management::story::StoryState::Retired,
    })
    .map_err(|e| format!("#{outcome_number} retire state-transition failed: {e}"))?;
    r.mutate_sync(BoardEvent::StoryClosed {
        meta: meta(),
        number: outcome_number,
        reason: Some(format!("retired: {reason}")),
    })
    .map_err(|e| format!("#{outcome_number} retire enqueue failed: {e}"))?;
    if !outcome
        .raw_labels
        .iter()
        .any(|l| l.eq_ignore_ascii_case("retired"))
    {
        r.mutate_sync(BoardEvent::TagAdded {
            meta: meta(),
            number: outcome_number,
            label: "retired".to_string(),
        })
        .map_err(|e| format!("#{outcome_number} retire-tag enqueue failed: {e}"))?;
    }

    let orphaned = r
        .all_stories()
        .iter()
        .filter(|s| {
            crate::product_management::board::adapters::github::outcome_membership_from_labels(
                &s.raw_labels,
            )
            .is_some_and(|(owner, _)| owner == outcome_number)
        })
        .count();
    let mapping_note = match orphaned {
        0 => String::new(),
        n => format!(" — {n} iteration(s) now unmapped (re-home: outcome add ...)"),
    };
    Ok(format!(
        "#{outcome_number} {} retired (reason: {reason}){mapping_note}; it no longer appears in bpmux roadmap",
        outcome.title
    ))
}
