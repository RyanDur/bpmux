use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::blockers::{
    IterationCloseBlocker, broadcast_iteration_close_blocked,
    build_close_iteration_ready_queue_blocked_body, collect_ready_queue_blockers,
};
use super::notify::broadcast_iteration_closed;
use crate::hub;
use crate::transport::rest::shared::{
    broadcast_board_invalidate, broadcast_stories, err_body, json_headers,
};

#[derive(Default, Deserialize)]
pub struct CloseIterationRequest {
    #[serde(default)]
    pub iteration_id: Option<u64>,
}

pub(crate) enum CloseIterationError {
    NotAnIteration(u64),
    NotFound(u64),
    AlreadyClosed(u64),
    ReadyQueueBlocked(Vec<IterationCloseBlocker>),
    Canonical(String),
}

pub async fn post_close_iteration(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(dispatch): axum::Extension<crate::store::Dispatch>,
    body: Option<axum::Json<CloseIterationRequest>>,
) -> impl IntoResponse {
    let Some(iteration_id) = body.and_then(|b| b.0.iteration_id) else {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("close_iteration requires iteration_id (canonical Iteration issue#)."),
        );
    };
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(
                "close_iteration(canonical) requires refresh-driver — server has no canonical store available",
            ),
        );
    };
    match close_iteration(&hub, refresh, &dispatch, iteration_id).await {
        Ok(()) => {
            broadcast_board_invalidate(&hub);
            broadcast_stories(&hub, refresh);
            let body = serde_json::json!({
                "promoted": Vec::<u64>::new(),
                "deleted_marker_id": serde_json::Value::Null,
                "iteration_id": iteration_id,
            });
            (StatusCode::OK, json_headers(), body.to_string())
        }
        Err(CloseIterationError::NotAnIteration(id)) => (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body(&format!("close_iteration: #{id} is not an Iteration")),
        ),
        Err(CloseIterationError::NotFound(id)) => (
            StatusCode::NOT_FOUND,
            json_headers(),
            err_body(&format!("close_iteration: iteration_id #{id} not found")),
        ),
        Err(CloseIterationError::AlreadyClosed(id)) => (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body(&format!(
                "close_iteration: iteration #{id} is already closed"
            )),
        ),
        Err(CloseIterationError::ReadyQueueBlocked(blockers)) => {
            let body = build_close_iteration_ready_queue_blocked_body(&blockers);
            broadcast_iteration_close_blocked(&hub, &blockers);
            (StatusCode::CONFLICT, json_headers(), body)
        }
        Err(CloseIterationError::Canonical(close_err)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("close_iteration(canonical) failed: {close_err}")),
        ),
    }
}

pub(crate) async fn close_iteration(
    hub: &hub::Hub,
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
    dispatch: &crate::store::Dispatch,
    iteration_id: u64,
) -> Result<(), CloseIterationError> {
    let iteration = match refresh.iteration(crate::product_management::iteration::IterationId::new(
        iteration_id,
    )) {
        Some(it) => it,
        None => {
            return Err(if refresh.story(iteration_id).is_some() {
                CloseIterationError::NotAnIteration(iteration_id)
            } else {
                CloseIterationError::NotFound(iteration_id)
            });
        }
    };
    if iteration.complete {
        return Err(CloseIterationError::AlreadyClosed(iteration_id));
    }
    let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
    if !blockers.is_empty() {
        return Err(CloseIterationError::ReadyQueueBlocked(blockers));
    }
    close_iteration_canonical(refresh, dispatch, iteration_id)
        .await
        .map_err(CloseIterationError::Canonical)?;
    broadcast_iteration_closed(hub);
    Ok(())
}

pub(crate) async fn close_iteration_canonical(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
    dispatch: &crate::store::Dispatch,
    iteration_id: u64,
) -> Result<(), String> {
    let meta = || crate::product_management::story::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
    };
    refresh
        .mutate_sync(crate::product_management::story::events::StoryEvent::new(
            meta(),
            crate::product_management::story::StoryId::new(iteration_id),
            crate::product_management::story::events::StoryEventKind::CurrentIterationCleared,
        ))
        .map_err(|e| format!("#{iteration_id} current-designation-clear enqueue failed: {e}"))?;
    refresh
        .mutate_sync(crate::product_management::story::events::StoryEvent::new(
            meta(),
            crate::product_management::story::StoryId::new(iteration_id),
            crate::product_management::story::events::StoryEventKind::StoryClosed {
                reason: Some(
                    "iteration-close (terminal; promote-on-close retired #1556)".to_string(),
                ),
            },
        ))
        .map_err(|e| format!("#{iteration_id} iteration-close enqueue failed: {e}"))?;
    if let Some(iteration) = refresh.iteration(
        crate::product_management::iteration::IterationId::new(iteration_id),
    ) {
        dispatch.dispatch(crate::store::AppAction::Iteration(
            crate::product_management::iteration::IterationAction::Upserted(vec![
                crate::product_management::iteration::Iteration {
                    complete: true,
                    is_current: false,
                    ..iteration
                },
            ]),
        ));
    }
    Ok(())
}
