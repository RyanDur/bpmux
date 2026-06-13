use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{TrackerHandle, broadcast_board_invalidate, err_body, json_headers};
use crate::session;

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
        Some(s) if matches!(s.story_type, crate::board::domain::StoryType::Iteration) => s,
        Some(s) => {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body(&format!(
                    "close_iteration: #{iteration_number} is not an Iteration (story_type={:?})",
                    s.story_type
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
    let payload = session::message::StateDeltaPayload::IterationClose {
        promoted: Vec::new(),
        deleted_marker_id: None,
    };
    let delta_msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Some(t) = tracker.as_ref() {
        let body = "iteration-close: terminal close, promote-on-close retired (#1556).";
        if let Err(e) = t.post_story_comment(iteration_number, body).await {
            session::diag::log_diag(&format!(
                "[#1556] comment_on_story #{iteration_number} (close) failed: {e}"
            ));
        }
    }
    if let Err(e) = session.broadcast(&delta_msg) {
        session::diag::log_diag(&format!(
            "post_close_iteration_canonical: state-delta broadcast failed: {e}"
        ));
    }
    broadcast_board_invalidate(&session);
    super::shared::broadcast_board_snapshot(&session, refresh);
    let body = serde_json::json!({
        "promoted": Vec::<u64>::new(),
        "deleted_marker_id": serde_json::Value::Null,
        "iteration_number": iteration_number,
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub(crate) async fn collect_ready_queue_blockers(
    refresh: &dyn crate::board::RefreshDriver,
) -> Vec<IterationCloseBlocker> {
    refresh
        .all_stories()
        .into_iter()
        .filter(|s| matches!(s.state, crate::board::domain::StoryState::Ready) && !s.is_closed)
        .map(|s| IterationCloseBlocker {
            number: s.number,
            title: s.title,
            state: "ready".to_string(),
        })
        .collect()
}

fn build_close_iteration_ready_queue_blocked_body(blockers: &[IterationCloseBlocker]) -> String {
    serde_json::json!({
        "error": "close_iteration blocked: ready queue must be empty before canonical-via-GH iteration close (#1258 r3 guard).",
        "blockers": blockers,
    })
    .to_string()
}

pub(crate) async fn close_iteration_canonical(
    refresh: &Arc<dyn crate::board::RefreshDriver>,
    iteration_number: u64,
) -> Result<(), String> {
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    refresh
        .mutate_sync(crate::board::BoardEvent::CurrentIterationCleared {
            meta: meta(),
            number: iteration_number,
        })
        .map_err(|e| {
            format!("#{iteration_number} current-designation-clear enqueue failed: {e}")
        })?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta: meta(),
            number: iteration_number,
            reason: Some("iteration-close (terminal; promote-on-close retired #1556)".to_string()),
        })
        .map_err(|e| format!("#{iteration_number} iteration-close enqueue failed: {e}"))?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct IterationCloseBlocker {
    pub number: u64,
    pub title: String,
    pub state: String,
}

fn broadcast_iteration_close_blocked(
    session: &Arc<session::Session>,
    blockers: &[IterationCloseBlocker],
) {
    let payload = session::message::StateDeltaPayload::IterationCloseBlocked {
        blockers: blockers
            .iter()
            .map(|b| session::message::IterationCloseBlockerPayload {
                number: b.number,
                title: b.title.clone(),
                state: b.state.clone(),
            })
            .collect(),
    };
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!(
            "post_close_iteration: iteration-close-blocked broadcast failed: {e}"
        ));
    }
}
