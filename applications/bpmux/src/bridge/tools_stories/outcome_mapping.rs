use super::helpers::{
    ensure_outcome_in_canonical_store, ensure_story_in_canonical_store, require_purpose,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::outcome::Outcome;
use std::sync::Arc;

async fn verify_iteration_mapped(
    r: &Arc<dyn RefreshDriver>,
    outcome_number: u64,
    iteration_number: u64,
) -> Result<Outcome, String> {
    let outcome = ensure_outcome_in_canonical_store(r, outcome_number).await?;
    ensure_story_in_canonical_store(r, iteration_number).await?;
    if !outcome.iterations.contains(&iteration_number) {
        return Err(format!(
            "iteration #{iteration_number} is not mapped to outcome #{outcome_number}"
        ));
    }
    Ok(outcome)
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
    let outcome = verify_iteration_mapped(r, outcome_number, iteration_number).await?;

    let mut parent = outcome
        .content
        .body
        .as_deref()
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default();
    parent
        .iterations
        .retain(|number| *number != iteration_number);
    let outcome_body = parent.write_into_body(outcome.content.body.as_deref().unwrap_or_default());
    r.update_body(outcome_number, outcome_body)
        .await
        .map_err(|e| format!("#{outcome_number} iteration-unmap body update failed: {e}"))?;

    let iteration = ensure_story_in_canonical_store(r, iteration_number).await?;
    let mut child = iteration
        .body()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    child.outcome = None;
    let iteration_body = child.write_into_body(iteration.body().unwrap_or_default());
    r.update_body(iteration_number, iteration_body)
        .await
        .map_err(|e| format!("#{iteration_number} outcome-FK-clear body update failed: {e}"))?;

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
    let outcome = verify_iteration_mapped(r, outcome_number, iteration_number).await?;

    let band_size = reorder_iterations_within_outcome(
        r,
        outcome_number,
        outcome.content.body.as_deref(),
        iteration_number,
        position,
    )
    .await?;
    Ok(format!(
        "iteration #{iteration_number} reordered to priority {position} of {band_size} in outcome #{outcome_number}"
    ))
}

async fn reorder_iterations_within_outcome(
    r: &Arc<dyn RefreshDriver>,
    outcome_number: u64,
    outcome_body: Option<&str>,
    iteration_number: u64,
    requested_position: u32,
) -> Result<u32, String> {
    let mut parent = outcome_body
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default();

    let band_size = parent.iterations.len() as u32;
    parent
        .iterations
        .retain(|number| *number != iteration_number);
    let clamped = requested_position.clamp(1, band_size.max(1));
    let insert_index = (clamped as usize - 1).min(parent.iterations.len());
    parent.iterations.insert(insert_index, iteration_number);

    let body = parent.write_into_body(outcome_body.unwrap_or_default());
    r.update_body(outcome_number, body)
        .await
        .map_err(|e| format!("#{outcome_number} outcome-reorder body update failed: {e}"))?;
    Ok(band_size)
}
