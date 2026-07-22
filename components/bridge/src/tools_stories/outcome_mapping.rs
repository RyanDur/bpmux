use super::helpers::{
    ensure_iteration_in_canonical_store, ensure_outcome_in_canonical_store, require_purpose,
};
use product_management::iteration::IterationId;
use product_management::outcome::Outcome;
use source_codec::mapper::outcome_with_body;
use source_codec::relationships::{IterationRelationships, OutcomeRelationships, Relationships};
use std::sync::Arc;
use sync::SyncOrchestrator;

async fn verify_iteration_mapped(
    driver: &Arc<SyncOrchestrator>,
    select: &store::Select,
    outcome_id: u64,
    iteration_id: u64,
) -> Result<Outcome, String> {
    let outcome = ensure_outcome_in_canonical_store(driver, select, outcome_id).await?;
    ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    if !outcome.iterations.contains(&IterationId::new(iteration_id)) {
        return Err(format!(
            "iteration #{iteration_id} is not mapped to outcome #{outcome_id}"
        ));
    }
    Ok(outcome)
}

pub async fn handle_unmap_outcome_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &store::Dispatch,
    select: &store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "unmap an iteration from an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;

    let driver = refresh.ok_or("unmap_outcome_iteration requires refresh")?;
    let outcome = verify_iteration_mapped(driver, select, outcome_id, iteration_id).await?;

    let mut parent = outcome
        .body
        .as_deref()
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default();
    parent
        .iterations
        .retain(|id| *id != IterationId::new(iteration_id));
    let outcome_body = parent.write_into_body(outcome.body.as_deref().unwrap_or_default());
    let optimistic = outcome_with_body(&outcome, &outcome_body);
    driver
        .update_body(outcome_id, outcome_body)
        .map_err(|e| format!("#{outcome_id} iteration-unmap body update failed: {e}"))?;
    dispatch.dispatch(store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![optimistic]),
    ));

    let iteration = ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    let mut child = iteration
        .body
        .as_deref()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    child.outcome = None;
    let iteration_body = child.write_into_body(iteration.body.as_deref().unwrap_or_default());
    driver
        .update_body(iteration_id, iteration_body.clone())
        .map_err(|e| format!("#{iteration_id} outcome-FK-clear body update failed: {e}"))?;
    dispatch.dispatch(store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![
            product_management::iteration::Iteration {
                outcome: None,
                body: Some(iteration_body),
                ..iteration
            },
        ]),
    ));

    Ok(format!(
        "iteration #{iteration_id} unmapped from outcome #{outcome_id}; the iteration itself is untouched"
    ))
}

pub async fn handle_reorder_outcome_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&Arc<SyncOrchestrator>>,
    dispatch: &store::Dispatch,
    select: &store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "reorder an outcome's iterations")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'outcome_id' argument")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing 'iteration_id' argument")?;
    let position = args
        .get("position")
        .and_then(serde_json::Value::as_u64)
        .ok_or("outcome reorder needs --to <P> (the new 1-indexed priority-order)")?
        as u32;

    let driver = refresh.ok_or("reorder_outcome_iteration requires refresh")?;
    let outcome = verify_iteration_mapped(driver, select, outcome_id, iteration_id).await?;

    let (band_size, new_body) = reorder_iterations_within_outcome(
        driver,
        outcome_id,
        outcome.body.as_deref(),
        iteration_id,
        position,
    )
    .await?;
    dispatch.dispatch(store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![outcome_with_body(
            &outcome, &new_body,
        )]),
    ));
    Ok(format!(
        "iteration #{iteration_id} reordered to priority {position} of {band_size} in outcome #{outcome_id}"
    ))
}

async fn reorder_iterations_within_outcome(
    driver: &Arc<SyncOrchestrator>,
    outcome_id: u64,
    outcome_body: Option<&str>,
    iteration_id: u64,
    requested_position: u32,
) -> Result<(u32, String), String> {
    let mut parent = outcome_body
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default();

    let band_size = parent.iterations.len() as u32;
    parent
        .iterations
        .retain(|id| *id != IterationId::new(iteration_id));
    let clamped = requested_position.clamp(1, band_size.max(1));
    let insert_index = (clamped as usize - 1).min(parent.iterations.len());
    parent
        .iterations
        .insert(insert_index, IterationId::new(iteration_id));

    let body = parent.write_into_body(outcome_body.unwrap_or_default());
    driver
        .update_body(outcome_id, body.clone())
        .map_err(|e| format!("#{outcome_id} outcome-reorder body update failed: {e}"))?;
    Ok((band_size, body))
}
