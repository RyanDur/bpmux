use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::blockers::{
    broadcast_iteration_close_blocked, build_close_iteration_ready_queue_blocked_body,
    collect_ready_queue_blockers,
};
use crate::session;
use crate::transport::rest::shared::{
    TrackerHandle, broadcast_board_invalidate, err_body, json_headers,
};

#[derive(Default, Deserialize)]
pub struct CloseIterationRequest {
    #[serde(default)]
    pub iteration_id: Option<u64>,
}

pub async fn post_close_iteration(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(dispatch): axum::Extension<crate::store::Dispatch>,
    body: Option<axum::Json<CloseIterationRequest>>,
) -> impl IntoResponse {
    let iter_n = match body.and_then(|b| b.0.iteration_id) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body("close_iteration requires iteration_id (canonical Iteration issue#)."),
            );
        }
    };
    post_close_iteration_canonical(session, tracker, orchestrator, dispatch, iter_n).await
}

async fn post_close_iteration_canonical(
    session: Arc<session::Session>,
    tracker: TrackerHandle,
    orchestrator: crate::sync::OrchestratorHandle,
    dispatch: crate::store::Dispatch,
    iteration_id: u64,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(
                "close_iteration(canonical) requires refresh-driver — server has no canonical store available",
            ),
        );
    };
    let iteration = match refresh.iteration(crate::product_management::iteration::IterationId::new(
        iteration_id,
    )) {
        Some(it) => it,
        None => {
            if refresh.story(iteration_id).is_some() {
                return (
                    StatusCode::BAD_REQUEST,
                    json_headers(),
                    err_body(&format!(
                        "close_iteration: #{iteration_id} is not an Iteration"
                    )),
                );
            }
            return (
                StatusCode::NOT_FOUND,
                json_headers(),
                err_body(&format!(
                    "close_iteration: iteration_id #{iteration_id} not found"
                )),
            );
        }
    };
    if iteration.complete {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body(&format!(
                "close_iteration: iteration #{iteration_id} is already closed"
            )),
        );
    }
    let ready_queue_blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
    if !ready_queue_blockers.is_empty() {
        let body = build_close_iteration_ready_queue_blocked_body(&ready_queue_blockers);
        broadcast_iteration_close_blocked(&session, &ready_queue_blockers);
        return (StatusCode::CONFLICT, json_headers(), body);
    }
    if let Err(close_err) = close_iteration_canonical(refresh, &dispatch, iteration_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(&format!("close_iteration(canonical) failed: {close_err}")),
        );
    }
    let payload = crate::message::StateDeltaPayload::IterationClose {
        promoted: Vec::new(),
        deleted_marker_id: None,
    };
    let delta_msg = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(payload),
        ..crate::message::Message::default()
    };
    if let Some(t) = tracker.as_ref() {
        let body = "iteration-close: terminal close, promote-on-close retired (#1556).";
        if let Err(e) = t.post_story_comment(iteration_id, body).await {
            crate::diag::log_diag(&format!(
                "[#1556] comment_on_story #{iteration_id} (close) failed: {e}"
            ));
        }
    }
    if let Err(e) = session.broadcast(&delta_msg) {
        crate::diag::log_diag(&format!(
            "post_close_iteration_canonical: state-delta broadcast failed: {e}"
        ));
    }
    broadcast_board_invalidate(&session);
    crate::transport::rest::shared::broadcast_stories(&session, refresh);
    let body = serde_json::json!({
        "promoted": Vec::<u64>::new(),
        "deleted_marker_id": serde_json::Value::Null,
        "iteration_id": iteration_id,
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub(crate) async fn close_iteration_canonical(
    refresh: &Arc<dyn crate::sync::RefreshDriver>,
    dispatch: &crate::store::Dispatch,
    iteration_id: u64,
) -> Result<(), String> {
    let meta = || crate::product_management::story::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::story::peer::PeerId::local(),
    };
    refresh
        .mutate_sync(
            crate::product_management::story::events::StoryEvent::CurrentIterationCleared {
                meta: meta(),
                id: crate::product_management::story::StoryId::new(iteration_id),
            },
        )
        .map_err(|e| format!("#{iteration_id} current-designation-clear enqueue failed: {e}"))?;
    refresh
        .mutate_sync(
            crate::product_management::story::events::StoryEvent::StoryClosed {
                meta: meta(),
                id: crate::product_management::story::StoryId::new(iteration_id),
                reason: Some(
                    "iteration-close (terminal; promote-on-close retired #1556)".to_string(),
                ),
            },
        )
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
