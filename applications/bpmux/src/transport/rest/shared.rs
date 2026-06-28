use std::sync::Arc;

use serde::Deserialize;

use crate::session;

pub type TrackerHandle = Option<Arc<crate::integrations::github::GitHubSourceAdapter>>;

#[derive(Deserialize)]
pub struct CommentPayload {
    pub body: String,
}

pub(super) fn err_body(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

pub(crate) fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}

pub(crate) fn broadcast_board_invalidate(session: &session::Session) {
    let delta = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rest: board-invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn board_snapshot_message(stories: serde_json::Value) -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardSnapshot { stories }),
        ..crate::message::Message::default()
    }
}

pub(crate) fn broadcast_board_snapshot(
    session: &session::Session,
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) {
    let stories = serde_json::Value::Array(super::board::ordered_board_stories(refresh));
    let delta = board_snapshot_message(stories);
    session.broadcast_ephemeral(&delta);
}

pub(crate) fn outcomes_message(outcomes: serde_json::Value) -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::Outcomes { outcomes }),
        ..crate::message::Message::default()
    }
}

pub(crate) fn broadcast_outcomes(
    session: &session::Session,
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) {
    let outcomes = serde_json::Value::Array(super::board::board_outcomes(refresh));
    session.broadcast_ephemeral(&outcomes_message(outcomes));
}

pub(crate) fn iterations_message(iterations: serde_json::Value) -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::Iterations { iterations }),
        ..crate::message::Message::default()
    }
}

pub(crate) fn broadcast_iterations(
    session: &session::Session,
    refresh: &Arc<dyn crate::product_management::board::RefreshDriver>,
) {
    let iterations = serde_json::Value::Array(super::board::board_iterations(refresh));
    session.broadcast_ephemeral(&iterations_message(iterations));
}
