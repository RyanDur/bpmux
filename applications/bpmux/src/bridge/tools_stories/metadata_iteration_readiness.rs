use super::event_meta::meta_for;
use super::helpers::{ensure_iteration_in_canonical_store, require_purpose, require_purpose_any};
use product_management::iteration::{Iteration, IterationId};
use product_management::story::StoryId;

pub(crate) async fn handle_designate_current_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<crate::sync::SyncOrchestrator>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose(role, &["pm"], "designate current iteration")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let driver = refresh.ok_or("designate_current_iteration requires refresh")?;
    let iteration = ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    if iteration.complete {
        return Err(format!(
            "iteration #{iteration_id} is closed; a closed iteration cannot be the current iteration"
        ));
    }
    let meta = || meta_for(agent_name, actor_id);
    let prior: Vec<product_management::iteration::Iteration> = select
        .select(|state| product_management::iteration::all(&state.iterations))
        .into_iter()
        .filter(|iteration| iteration.is_current && iteration.id != IterationId::new(iteration_id))
        .collect();
    for iteration in &prior {
        driver
            .mutate(product_management::story::events::StoryEvent::new(
                meta(),
                StoryId::new(iteration.id.value()),
                product_management::story::events::StoryEventKind::CurrentIterationCleared,
            ))
            .map_err(|e| {
                format!(
                    "#{} current-designation-clear enqueue failed: {e}",
                    iteration.id.value()
                )
            })?;
    }
    driver
        .mutate(product_management::story::events::StoryEvent::new(
            meta(),
            StoryId::new(iteration_id),
            product_management::story::events::StoryEventKind::CurrentIterationDesignated,
        ))
        .map_err(|e| format!("#{iteration_id} current-designation enqueue failed: {e}"))?;
    let mut updates: Vec<product_management::iteration::Iteration> = prior
        .into_iter()
        .map(|it| product_management::iteration::Iteration {
            is_current: false,
            ..it
        })
        .collect();
    updates.push(product_management::iteration::Iteration {
        is_current: true,
        ..iteration
    });
    dispatch.dispatch(crate::store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(updates),
    ));
    Ok(format!("iteration #{iteration_id} designated current"))
}

pub(crate) async fn handle_close_pre_pickup_review(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<crate::sync::SyncOrchestrator>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose_any(
        role,
        product_management::iteration::readiness::ITERATION_REVIEWER_PURPOSE,
        "close the pre-pickup review",
    )?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let driver = refresh.ok_or("close_pre_pickup_review requires refresh")?;
    let iteration = ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    let role = role.to_string();
    let label = source_codec::gh_label_codec::GhLabelCodec.encode_iteration_review_closed(&role);
    let meta = meta_for(agent_name, actor_id);
    driver
        .mutate(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(iteration_id),
            product_management::story::events::StoryEventKind::TagAdded { label },
        ))
        .map_err(|e| format!("#{iteration_id} review-close enqueue failed: {e}"))?;
    let mut review_closers = iteration.review_closers.clone();
    if !review_closers.contains(&role) {
        review_closers.push(role.clone());
    }
    dispatch.dispatch(crate::store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![Iteration {
            review_closers,
            ..iteration
        }]),
    ));
    Ok(format!(
        "pre-pickup review for iteration #{iteration_id} closed by {role}"
    ))
}

pub(crate) async fn handle_dev_ok_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<crate::sync::SyncOrchestrator>>,
    dispatch: &crate::store::Dispatch,
    select: &crate::store::Select,
) -> Result<String, String> {
    require_purpose_any(
        role,
        product_management::iteration::readiness::ITERATION_BUILDER_PURPOSE,
        "record dev-OK for the iteration",
    )?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let driver = refresh.ok_or("dev_ok_iteration requires refresh")?;
    let iteration = ensure_iteration_in_canonical_store(driver, select, iteration_id).await?;
    let role = role.to_string();
    let label = source_codec::gh_label_codec::GhLabelCodec.encode_iteration_dev_ok(&role);
    let meta = meta_for(agent_name, actor_id);
    driver
        .mutate(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(iteration_id),
            product_management::story::events::StoryEventKind::TagAdded { label },
        ))
        .map_err(|e| format!("#{iteration_id} dev-OK enqueue failed: {e}"))?;
    let mut dev_ok_by = iteration.dev_ok_by.clone();
    if !dev_ok_by.contains(&role) {
        dev_ok_by.push(role.clone());
    }
    dispatch.dispatch(crate::store::AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![Iteration {
            dev_ok_by,
            ..iteration
        }]),
    ));
    Ok(format!("iteration #{iteration_id} dev-OK'd by {role}"))
}
