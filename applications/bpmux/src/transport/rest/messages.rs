use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::hub;

pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

#[derive(Deserialize)]
pub struct PostMessage {
    pub from: String,
    pub text: String,
    pub message_type: Option<String>,
    #[serde(default)]
    pub addressee: Option<String>,
    #[serde(default)]
    pub multi_depth: Option<crate::message::MultiDepthPayload>,
    #[serde(default)]
    pub to: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GetMessagesParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub chat_only: Option<bool>,
    #[serde(rename = "as")]
    pub as_peer: Option<String>,
    pub message_type: Option<String>,
    pub before: Option<String>,
}

pub async fn get_messages(
    State(hub): State<Arc<hub::Hub>>,
    Query(params): Query<GetMessagesParams>,
) -> impl IntoResponse {
    let chat_only = params.chat_only.unwrap_or(true);
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT);
    let wanted_type = params
        .message_type
        .as_deref()
        .map(|mt| crate::message::MessageType::from(Some(mt)));
    let before_ts = params
        .before
        .as_deref()
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok());
    let as_peer = params.as_peer.clone();
    let matches = |m: &crate::message::Message| -> bool {
        let type_ok = match &wanted_type {
            Some(t) => m.message_type == *t,
            None => !chat_only || m.is_chat(),
        };
        if !type_ok {
            return false;
        }
        if let Some(b) = before_ts
            && m.ts >= b
        {
            return false;
        }
        if let Some(ref peer) = as_peer
            && !crate::transport::ws::should_deliver(m, peer)
        {
            return false;
        }
        true
    };
    match hub.read_recent(limit, offset, &matches) {
        Ok(page) => (
            StatusCode::OK,
            serde_json::to_string(&page).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read: {e}"),
        ),
    }
}

pub async fn post_message(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(routing): axum::Extension<Arc<crate::routing::RoutingStore>>,
    Json(payload): Json<PostMessage>,
) -> impl IntoResponse {
    if payload.text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "text must not be empty".to_string(),
        );
    }
    if payload.text.len() > 65536 {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "message too large: {} bytes exceeds limit of 65536",
                payload.text.len()
            ),
        );
    }
    if payload.message_type.as_deref() == Some("system") {
        return (
            StatusCode::FORBIDDEN,
            "message_type=\"system\" is reserved for internal callers".to_string(),
        );
    }
    let explicit_privacy = payload.to.is_some();
    let sender_id = routing
        .resolve(crate::routing::short_name(&payload.from))
        .into_iter()
        .next();
    let msg = crate::message::Message {
        id: String::new(),
        from: payload.from,
        sender_id,
        text: payload.text,
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::from(payload.message_type.as_deref()),
        to: payload.to,
        addressee: payload.addressee,
        explicit_privacy,
        multi_depth: payload.multi_depth,
        state_delta: None,
        state_snapshot: None,
    };
    if let Err(e) = hub.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
}
