use super::helpers::{ensure_story_in_canonical_store, require_purpose};
use crate::board::RefreshDriver;
use crate::board::domain::StoryType;
use crate::board::events::EventMeta;
use crate::board::{BoardEvent, PeerId, Story};
use std::sync::Arc;

fn meta_for(agent_name: &str) -> EventMeta {
    EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    }
}

async fn mapped_iteration(
    r: &Arc<dyn RefreshDriver>,
    outcome_number: u64,
    iteration_number: u64,
) -> Result<Story, String> {
    let outcome = ensure_story_in_canonical_store(r, outcome_number).await?;
    if outcome.story_type != StoryType::Outcome {
        return Err(format!(
            "#{outcome_number} is not an Outcome (story_type={:?}); it may be a story, not an outcome — outcomes: bpmux roadmap",
            outcome.story_type
        ));
    }
    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    if iteration
        .outcome_membership
        .is_none_or(|(owner, _)| owner != outcome_number)
    {
        return Err(format!(
            "iteration #{iteration_number} is not mapped to outcome #{outcome_number}"
        ));
    }
    Ok(iteration)
}

pub(crate) async fn handle_unmap_outcome_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "unmap an iteration from an outcome")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;

    let r = refresh.ok_or("unmap_outcome_iteration requires refresh (RefreshDriver)")?;
    mapped_iteration(r, outcome_number, iteration_number).await?;

    r.mutate_sync(BoardEvent::OutcomeMembershipCleared {
        meta: meta_for(agent_name),
        number: iteration_number,
    })
    .map_err(|e| format!("#{iteration_number} outcome-unmap enqueue failed: {e}"))?;
    Ok(format!(
        "iteration #{iteration_number} unmapped from outcome #{outcome_number}; the iteration itself is untouched"
    ))
}

pub(crate) async fn handle_reorder_outcome_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "reorder an outcome's iterations")?;
    let outcome_number = args
        .get("outcome_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_number' argument")?;
    let iteration_number = args
        .get("iteration_number")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_number' argument")?;
    let position = args
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .ok_or("outcome reorder needs --to <P> (the new 1-indexed priority-order)")?
        as u32;

    let r = refresh.ok_or("reorder_outcome_iteration requires refresh (RefreshDriver)")?;
    mapped_iteration(r, outcome_number, iteration_number).await?;

    r.mutate_sync(BoardEvent::OutcomeMembershipSet {
        meta: meta_for(agent_name),
        number: iteration_number,
        outcome_number,
        position,
    })
    .map_err(|e| format!("#{iteration_number} outcome-reorder enqueue failed: {e}"))?;
    Ok(format!(
        "iteration #{iteration_number} reordered to priority {position} in outcome #{outcome_number}"
    ))
}
