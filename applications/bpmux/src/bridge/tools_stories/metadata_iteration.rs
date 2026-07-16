use super::helpers::{
    ensure_iteration_in_canonical_store, ensure_outcome_in_canonical_store,
    ensure_story_in_canonical_store, require_purpose,
};
use product_management::iteration::IterationId;
use product_management::outcome::OutcomeId;
use source_codec::mapper::outcome_with_body;
use source_codec::relationships::{IterationRelationships, OutcomeRelationships, Relationships};

pub(crate) async fn handle_add_story_to_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition =
        workflow::resolve_transition(workflow, "add-to-iteration", workflow::GateClass::None)
            .map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    let story_id = args
        .get("story_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_id' argument")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as u32;
    if story_id == iteration_id {
        return Err(format!(
            "story #{story_id} cannot be added to itself as iteration-member"
        ));
    }
    let driver = refresh.ok_or("add_story_to_iteration requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(driver, select, story_id).await?;
    super::iteration_members::set_member_position(
        driver,
        select,
        dispatch,
        iteration_id,
        story_id,
        position,
    )
    .await?;
    Ok(format!(
        "#{story_id} added to iteration #{iteration_id} at position {position}"
    ))
}

pub(crate) async fn handle_map_iteration_to_outcome(
    args: &serde_json::Map<String, serde_json::Value>,
    role: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "map an iteration to an outcome")?;
    let outcome_id = args
        .get("outcome_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'outcome_id' argument")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as u32;
    let driver = refresh.ok_or("map_iteration_to_outcome requires refresh (RefreshDriver)")?;
    let iteration = ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    let outcome = ensure_outcome_in_canonical_store(driver, select, outcome_id).await?;

    let mut parent = outcome
        .body
        .as_deref()
        .and_then(OutcomeRelationships::read_from_body)
        .unwrap_or_default();
    parent
        .iterations
        .retain(|id| *id != IterationId::new(iteration_id));
    let insert_index = (position.max(1) as usize - 1).min(parent.iterations.len());
    parent
        .iterations
        .insert(insert_index, IterationId::new(iteration_id));
    let outcome_body = parent.write_into_body(outcome.body.as_deref().unwrap_or_default());
    let optimistic = outcome_with_body(&outcome, &outcome_body);
    driver
        .update_body(outcome_id, outcome_body)
        .await
        .map_err(|e| format!("#{outcome_id} iteration-map body update failed: {e}"))?;
    dispatch.dispatch(crate::store::AppAction::Outcome(
        product_management::outcome::OutcomeAction::Upserted(vec![optimistic]),
    ));

    let mut child = iteration
        .body
        .as_deref()
        .and_then(IterationRelationships::read_from_body)
        .unwrap_or_default();
    child.outcome = Some(OutcomeId::new(outcome_id));
    let iteration_body = child.write_into_body(iteration.body.as_deref().unwrap_or_default());
    driver
        .update_body(iteration_id, iteration_body.clone())
        .await
        .map_err(|e| format!("#{iteration_id} outcome-FK-set body update failed: {e}"))?;
    dispatch.dispatch(crate::store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![
            product_management::iteration::Iteration {
                outcome: Some(OutcomeId::new(outcome_id)),
                body: Some(iteration_body),
                ..iteration
            },
        ]),
    ));

    Ok(format!(
        "iteration #{iteration_id} maps to outcome #{outcome_id} at position {position}"
    ))
}
