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
    pub iteration_number: Option<u64>,
}

pub async fn post_close_iteration(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    body: Option<axum::Json<CloseIterationRequest>>,
) -> impl IntoResponse {
    let iter_n = match body.and_then(|b| b.0.iteration_number) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body("close_iteration requires iteration_number (canonical Iteration issue#)."),
            );
        }
    };
    post_close_iteration_canonical(session, tracker, test_refresh, iter_n).await
}

async fn post_close_iteration_canonical(
    session: Arc<session::Session>,
    tracker: TrackerHandle,
    test_refresh: crate::server::TestRefreshHandle,
    iteration_number: u64,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let Some(refresh) = test_refresh.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(
                "close_iteration(canonical) requires refresh-driver — server has no canonical store available",
            ),
        );
    };
    let iteration = match refresh.story(iteration_number) {
        Some(s)
            if matches!(
                crate::product_management::board::adapters::github::story_type_from_labels_or_title(
                    &s.raw_labels,
                    &s.title
                ),
                crate::product_management::story::StoryType::Iteration
            ) =>
        {
            s
        }
        Some(s) => {
            let story_type =
                crate::product_management::board::adapters::github::story_type_from_labels_or_title(
                    &s.raw_labels,
                    &s.title,
                );
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body(&format!(
                    "close_iteration: #{iteration_number} is not an Iteration (story_type={story_type:?})"
                )),
            );
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                json_headers(),
                err_body(&format!(
                    "close_iteration: iteration_number #{iteration_number} not found"
                )),
            );
        }
    };
    if iteration.is_closed {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body(&format!(
                "close_iteration: iteration #{iteration_number} is already closed"
            )),
        );
    }
    let ready_queue_blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
    if !ready_queue_blockers.is_empty() {
        let body = build_close_iteration_ready_queue_blocked_body(&ready_queue_blockers);
        broadcast_iteration_close_blocked(&session, &ready_queue_blockers);
        return (StatusCode::CONFLICT, json_headers(), body);
    }
    if let Err(close_err) = close_iteration_canonical(refresh, iteration_number).await {
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
        if let Err(e) = t.post_story_comment(iteration_number, body).await {
            crate::diag::log_diag(&format!(
                "[#1556] comment_on_story #{iteration_number} (close) failed: {e}"
            ));
        }
    }
    if let Err(e) = session.broadcast(&delta_msg) {
        crate::diag::log_diag(&format!(
            "post_close_iteration_canonical: state-delta broadcast failed: {e}"
        ));
    }
    broadcast_board_invalidate(&session);
    crate::transport::rest::shared::broadcast_board_snapshot(&session, refresh);
    let body = serde_json::json!({
        "promoted": Vec::<u64>::new(),
        "deleted_marker_id": serde_json::Value::Null,
        "iteration_number": iteration_number,
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub(crate) async fn close_iteration_canonical(
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
    iteration_number: u64,
) -> Result<(), String> {
    let meta = || crate::product_management::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    refresh
        .mutate_sync(
            crate::product_management::board::BoardEvent::CurrentIterationCleared {
                meta: meta(),
                number: iteration_number,
            },
        )
        .map_err(|e| {
            format!("#{iteration_number} current-designation-clear enqueue failed: {e}")
        })?;
    refresh
        .mutate_sync(crate::product_management::board::BoardEvent::StoryClosed {
            meta: meta(),
            number: iteration_number,
            reason: Some("iteration-close (terminal; promote-on-close retired #1556)".to_string()),
        })
        .map_err(|e| format!("#{iteration_number} iteration-close enqueue failed: {e}"))?;
    Ok(())
}
