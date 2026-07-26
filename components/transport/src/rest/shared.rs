use std::sync::Arc;

use serde::Deserialize;

pub type TrackerHandle = Option<Arc<integrations::github::GitHubSourceAdapter>>;

pub const OWNER_NONCE_HEADER: &str = "x-bpmux-owner-nonce";

#[derive(Deserialize)]
pub struct CommentPayload {
    pub body: String,
}

pub(super) fn err_body(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

pub(crate) type JsonResponse = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

pub(crate) fn owner_walk_or_forbidden(
    select: &store::Select,
    headers: &axum::http::HeaderMap,
    action: &str,
) -> Result<(), JsonResponse> {
    let claimed = select.select(|state| owner_session::claimed_nonce(&state.owner_session));
    let presented = headers
        .get(OWNER_NONCE_HEADER)
        .and_then(|value| value.to_str().ok());
    if owner_session::owner_walk_authorized(claimed.as_deref(), presented) {
        Ok(())
    } else {
        Err((
            axum::http::StatusCode::FORBIDDEN,
            json_headers(),
            err_body(&format!(
                "{action} requires the owner session — attach the current launch nonce"
            )),
        ))
    }
}

pub fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}

pub fn broadcast_board_invalidate(hub: &hub::Hub) {
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
        diag::log_diag(&format!(
            "rest: board-invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub fn stories_message(stories: serde_json::Value) -> message::Message {
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

pub fn broadcast_stories(hub: &hub::Hub, select: &store::Select) {
    let stories = serde_json::Value::Array(super::board::ordered_board_stories(select));
    hub.broadcast_ephemeral(&stories_message(stories));
}

pub fn epics_message(epics: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Epics { epics }),
        ..message::Message::default()
    }
}

pub fn broadcast_epics(hub: &hub::Hub, select: &store::Select) {
    let epics = serde_json::Value::Array(super::board::board_epics(select));
    hub.broadcast_ephemeral(&epics_message(epics));
}

pub fn outcomes_message(outcomes: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Outcomes { outcomes }),
        ..message::Message::default()
    }
}

pub fn broadcast_outcomes(hub: &hub::Hub, select: &store::Select) {
    let outcomes = serde_json::Value::Array(super::board::board_outcomes(select));
    hub.broadcast_ephemeral(&outcomes_message(outcomes));
}

pub fn iterations_message(iterations: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Iterations { iterations }),
        ..message::Message::default()
    }
}

pub fn broadcast_iterations(hub: &hub::Hub, select: &store::Select) {
    let iterations = serde_json::Value::Array(super::board::board_iterations(select));
    hub.broadcast_ephemeral(&iterations_message(iterations));
}

pub fn roadmap_message(roadmap: serde_json::Value) -> message::Message {
    message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: message::MessageType::StateDelta,
        state_delta: Some(message::StateDeltaPayload::Roadmap { roadmap }),
        ..message::Message::default()
    }
}

pub fn broadcast_roadmap(hub: &hub::Hub, select: &store::Select) {
    let roadmap = serde_json::Value::Array(super::board::board_roadmap(select));
    hub.broadcast_ephemeral(&roadmap_message(roadmap));
}
