use std::sync::Arc;

use serde::Deserialize;

use crate::session;

pub type TrackerHandle = Option<Arc<crate::board::adapters::github::GitHubSourceAdapter>>;

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
    let delta = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(session::message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        session::diag::log_diag(&format!(
            "rest: board-invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn board_snapshot_message(stories: serde_json::Value) -> session::Message {
    session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(session::message::StateDeltaPayload::BoardSnapshot { stories }),
        ..session::Message::default()
    }
}

pub(crate) fn broadcast_board_snapshot(
    session: &session::Session,
    refresh: &Arc<dyn crate::board::RefreshDriver>,
) {
    let stories = serde_json::Value::Array(super::board::ordered_board_stories(refresh));
    let delta = board_snapshot_message(stories);
    if let Err(e) = session.broadcast(&delta) {
        session::diag::log_diag(&format!(
            "rest: board-snapshot state-delta broadcast failed: {e}"
        ));
    }
}
