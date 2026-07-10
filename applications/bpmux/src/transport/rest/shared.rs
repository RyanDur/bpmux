use std::sync::Arc;

use serde::Deserialize;

use crate::hub;

pub type TrackerHandle = Option<Arc<crate::integrations::github::GitHubSourceAdapter>>;

pub(crate) const OWNER_NONCE_HEADER: &str = "x-bpmux-owner-nonce";

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

pub(crate) fn broadcast_board_invalidate(hub: &hub::Hub) {
    let delta = message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..message::Message::default()
    };
    if let Err(e) = hub.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rest: board-invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn stories_message(stories: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Stories {
            stories,
            columns: Some(serde_json::Value::Array(super::board::board_columns_json())),
        }),
        ..message::Message::default()
    }
}

pub(crate) fn broadcast_stories(hub: &hub::Hub, refresh: &Arc<dyn crate::sync::RefreshDriver>) {
    let stories = serde_json::Value::Array(super::board::ordered_board_stories(refresh));
    hub.broadcast_ephemeral(&stories_message(stories));
}

pub(crate) fn epics_message(epics: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Epics { epics }),
        ..message::Message::default()
    }
}

pub(crate) fn broadcast_epics(hub: &hub::Hub, refresh: &Arc<dyn crate::sync::RefreshDriver>) {
    let epics = serde_json::Value::Array(super::board::board_epics(refresh));
    hub.broadcast_ephemeral(&epics_message(epics));
}

pub(crate) fn outcomes_message(outcomes: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Outcomes { outcomes }),
        ..message::Message::default()
    }
}

pub(crate) fn broadcast_outcomes(hub: &hub::Hub, refresh: &Arc<dyn crate::sync::RefreshDriver>) {
    let outcomes = serde_json::Value::Array(super::board::board_outcomes(refresh));
    hub.broadcast_ephemeral(&outcomes_message(outcomes));
}

pub(crate) fn iterations_message(iterations: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Iterations { iterations }),
        ..message::Message::default()
    }
}

pub(crate) fn broadcast_iterations(hub: &hub::Hub, refresh: &Arc<dyn crate::sync::RefreshDriver>) {
    let iterations = serde_json::Value::Array(super::board::board_iterations(refresh));
    hub.broadcast_ephemeral(&iterations_message(iterations));
}

pub(crate) fn roadmap_message(roadmap: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Roadmap { roadmap }),
        ..message::Message::default()
    }
}

pub(crate) fn broadcast_roadmap(hub: &hub::Hub, refresh: &Arc<dyn crate::sync::RefreshDriver>) {
    let roadmap = serde_json::Value::Array(super::board::board_roadmap(refresh));
    hub.broadcast_ephemeral(&roadmap_message(roadmap));
}
