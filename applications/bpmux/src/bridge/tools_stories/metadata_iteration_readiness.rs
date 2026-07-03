use super::helpers::{
    ensure_iteration_in_canonical_store, require_purpose, require_purpose_any, role_from_name,
};
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::StoryId;

pub(crate) async fn handle_designate_current_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose(agent_name, &["pm"], "designate current iteration")?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let r = refresh.ok_or("designate_current_iteration requires refresh (RefreshDriver)")?;
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    if iteration.complete {
        return Err(format!(
            "iteration #{iteration_id} is closed; a closed iteration cannot be the current iteration"
        ));
    }
    let meta = || crate::product_management::board::events::EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    let prior: Vec<crate::product_management::iteration::Iteration> = r
        .iterations()
        .into_iter()
        .filter(|it| it.is_current && it.id != IterationId::new(iteration_id))
        .collect();
    for it in &prior {
        r.mutate_sync(
            crate::product_management::board::StoryEvent::CurrentIterationCleared {
                meta: meta(),
                id: StoryId::new(it.id.value()),
            },
        )
        .map_err(|e| {
            format!(
                "#{} current-designation-clear enqueue failed: {e}",
                it.id.value()
            )
        })?;
    }
    r.mutate_sync(
        crate::product_management::board::StoryEvent::CurrentIterationDesignated {
            meta: meta(),
            id: StoryId::new(iteration_id),
        },
    )
    .map_err(|e| format!("#{iteration_id} current-designation enqueue failed: {e}"))?;
    let mut updates: Vec<crate::product_management::iteration::Iteration> = prior
        .into_iter()
        .map(|it| crate::product_management::iteration::Iteration {
            is_current: false,
            ..it
        })
        .collect();
    updates.push(crate::product_management::iteration::Iteration {
        is_current: true,
        ..iteration
    });
    dispatch.dispatch(crate::store::AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(updates),
    ));
    Ok(format!("iteration #{iteration_id} designated current"))
}

pub(crate) async fn handle_close_pre_pickup_review(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose_any(
        agent_name,
        crate::product_management::iteration::readiness::ITERATION_REVIEWER_PURPOSE,
        "close the pre-pickup review",
    )?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let r = refresh.ok_or("close_pre_pickup_review requires refresh (RefreshDriver)")?;
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    let role = role_from_name(agent_name).to_string();
    let label = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
        .encode_iteration_review_closed(&role);
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::StoryEvent::TagAdded {
        meta,
        id: StoryId::new(iteration_id),
        label,
    })
    .map_err(|e| format!("#{iteration_id} review-close enqueue failed: {e}"))?;
    r.drain_pending()
        .await
        .map_err(|e| format!("#{iteration_id} review-close push failed: {e}"))?;
    let mut review_closers = iteration.review_closers.clone();
    if !review_closers.contains(&role) {
        review_closers.push(role.clone());
    }
    dispatch.dispatch(crate::store::AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(vec![Iteration {
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
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    require_purpose_any(
        agent_name,
        crate::product_management::iteration::readiness::ITERATION_BUILDER_PURPOSE,
        "record dev-OK for the iteration",
    )?;
    let iteration_id = args
        .get("iteration_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'iteration_id' argument")?;
    let r = refresh.ok_or("dev_ok_iteration requires refresh (RefreshDriver)")?;
    let iteration = ensure_iteration_in_canonical_store(r, iteration_id).await?;
    let role = role_from_name(agent_name).to_string();
    let label = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
        .encode_iteration_dev_ok(&role);
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::routing::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::StoryEvent::TagAdded {
        meta,
        id: StoryId::new(iteration_id),
        label,
    })
    .map_err(|e| format!("#{iteration_id} dev-OK enqueue failed: {e}"))?;
    r.drain_pending()
        .await
        .map_err(|e| format!("#{iteration_id} dev-OK push failed: {e}"))?;
    let mut dev_ok_by = iteration.dev_ok_by.clone();
    if !dev_ok_by.contains(&role) {
        dev_ok_by.push(role.clone());
    }
    dispatch.dispatch(crate::store::AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(vec![Iteration {
            dev_ok_by,
            ..iteration
        }]),
    ));
    Ok(format!("iteration #{iteration_id} dev-OK'd by {role}"))
}
