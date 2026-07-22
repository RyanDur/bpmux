use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::bridge::iteration_close::{
    CloseIterationError, broadcast_iteration_close_blocked,
    build_close_iteration_ready_queue_blocked_body, close_iteration,
};
use crate::hub;
use crate::transport::rest::shared::{
    broadcast_board_invalidate, broadcast_stories, err_body, json_headers,
};

#[derive(Default, Deserialize)]
pub struct CloseIterationRequest {
    #[serde(default)]
    pub iteration_id: Option<u64>,
}

pub async fn post_close_iteration(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(dispatch): axum::Extension<crate::store::Dispatch>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
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
    match close_iteration(&hub, refresh, &dispatch, &select, iteration_id).await {
        Ok(()) => {
            broadcast_board_invalidate(&hub);
            broadcast_stories(&hub, &select);
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
