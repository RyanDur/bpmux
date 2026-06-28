use super::helpers::{
    ensure_outcome_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId};
use crate::product_management::iteration::Iteration;
use crate::product_management::story::Story;
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
    ensure_outcome_in_canonical_store(r, outcome_number).await?;
    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    if crate::product_management::board::adapters::github::outcome_membership_from_labels(
        &iteration.raw_labels,
    )
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

    let band_size = reorder_iterations_within_outcome(
        r,
        agent_name,
        outcome_number,
        iteration_number,
        position,
    )?;
    Ok(format!(
        "iteration #{iteration_number} reordered to priority {position} of {band_size} in outcome #{outcome_number}"
    ))
}

fn outcome_position_of(iteration: &Iteration) -> Option<u32> {
    iteration.outcome_membership.map(|(_, position)| position)
}

fn reorder_iterations_within_outcome(
    r: &Arc<dyn RefreshDriver>,
    agent_name: &str,
    outcome_number: u64,
    iteration_number: u64,
    requested_position: u32,
) -> Result<u32, String> {
    let all = r.all_stories();
    let mut building =
        crate::product_management::iteration::readiness::building_iterations(outcome_number, &all);
    building.sort_by_key(|iteration| {
        (
            outcome_position_of(iteration).unwrap_or(u32::MAX),
            iteration.id().saved_number().unwrap_or(u64::MAX),
        )
    });

    let band_size = building.len() as u32;
    let clamped = requested_position.clamp(1, band_size.max(1));
    let mut sequence: Vec<u64> = building
        .iter()
        .filter_map(|iteration| iteration.id().saved_number())
        .filter(|number| *number != iteration_number)
        .collect();
    let insert_index = (clamped as usize - 1).min(sequence.len());
    sequence.insert(insert_index, iteration_number);

    for (index, number) in sequence.iter().enumerate() {
        let new_position = (index + 1) as u32;
        let current = building
            .iter()
            .find(|iteration| iteration.id().saved_number() == Some(*number))
            .and_then(outcome_position_of);
        if current == Some(new_position) {
            continue;
        }
        r.mutate_sync(BoardEvent::OutcomeMembershipSet {
            meta: meta_for(agent_name),
            number: *number,
            outcome_number,
            position: new_position,
        })
        .map_err(|e| format!("#{number} outcome-reorder enqueue failed: {e}"))?;
    }
    Ok(band_size)
}
