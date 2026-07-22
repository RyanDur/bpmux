use std::sync::Arc;

use super::blockers::{IterationCloseBlocker, collect_ready_queue_blockers};
use super::notify::broadcast_iteration_closed;

#[derive(Debug)]
pub enum CloseIterationError {
    NotAnIteration(u64),
    NotFound(u64),
    AlreadyClosed(u64),
    ReadyQueueBlocked(Vec<IterationCloseBlocker>),
    Canonical(String),
}

pub async fn close_iteration(
    hub: &hub::Hub,
    refresh: &Arc<sync::SyncOrchestrator>,
    dispatch: &store::Dispatch,
    select: &store::Select,
    iteration_id: u64,
) -> Result<(), CloseIterationError> {
    let iteration = match select.select(|state| {
        product_management::iteration::get(
            &state.iterations,
            product_management::iteration::IterationId::new(iteration_id),
        )
    }) {
        Some(iteration) => iteration,
        None => {
            let story_exists = select.select(|state| {
                product_management::story::slice::get(
                    &state.stories,
                    product_management::story::StoryId::new(iteration_id),
                )
                .is_some()
            });
            return Err(if story_exists {
                CloseIterationError::NotAnIteration(iteration_id)
            } else {
                CloseIterationError::NotFound(iteration_id)
            });
        }
    };
    if iteration.complete {
        return Err(CloseIterationError::AlreadyClosed(iteration_id));
    }
    let blockers = collect_ready_queue_blockers(select);
    if !blockers.is_empty() {
        return Err(CloseIterationError::ReadyQueueBlocked(blockers));
    }
    close_iteration_canonical(refresh, dispatch, select, iteration_id)
        .await
        .map_err(CloseIterationError::Canonical)?;
    broadcast_iteration_closed(hub);
    Ok(())
}

pub async fn close_iteration_canonical(
    refresh: &Arc<sync::SyncOrchestrator>,
    dispatch: &store::Dispatch,
    select: &store::Select,
    iteration_id: u64,
) -> Result<(), String> {
    let meta = || product_management::story::events::EventMeta {
        actor: "pm".to_string(),
        actor_id: None,
        ts: chrono::Utc::now(),
    };
    refresh
        .mutate(product_management::story::events::StoryEvent::new(
            meta(),
            product_management::story::StoryId::new(iteration_id),
            product_management::story::events::StoryEventKind::CurrentIterationCleared,
        ))
        .map_err(|e| format!("#{iteration_id} current-designation-clear enqueue failed: {e}"))?;
    refresh
        .mutate(product_management::story::events::StoryEvent::new(
            meta(),
            product_management::story::StoryId::new(iteration_id),
            product_management::story::events::StoryEventKind::StoryClosed {
                reason: Some(
                    "iteration-close (terminal; promote-on-close retired #1556)".to_string(),
                ),
            },
        ))
        .map_err(|e| format!("#{iteration_id} iteration-close enqueue failed: {e}"))?;
    if let Some(iteration) = select.select(|state| {
        product_management::iteration::get(
            &state.iterations,
            product_management::iteration::IterationId::new(iteration_id),
        )
    }) {
        dispatch.dispatch(store::AppAction::Iteration(
            product_management::iteration::IterationAction::Upserted(vec![
                product_management::iteration::Iteration {
                    complete: true,
                    is_current: false,
                    ..iteration
                },
            ]),
        ));
    }
    Ok(())
}
