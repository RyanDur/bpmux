use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

#[derive(Deserialize)]
pub struct PostMessage {
    pub from: String,
    pub text: String,
    pub message_type: Option<String>,
    #[serde(default)]
    pub addressee: Option<String>,
    #[serde(default)]
    pub multi_depth: Option<session::message::MultiDepthPayload>,
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
    State(session): State<Arc<session::Session>>,
    Query(params): Query<GetMessagesParams>,
) -> impl IntoResponse {
    match session.read_messages() {
        Ok(messages) => {
            let chat_only = params.chat_only.unwrap_or(true);
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT);
            let mut filtered: Vec<_> = if let Some(ref mt) = params.message_type {
                let wanted = session::MessageType::from(Some(mt.as_str()));
                messages
                    .into_iter()
                    .filter(|m| m.message_type == wanted)
                    .collect()
            } else if chat_only {
                messages.into_iter().filter(|m| m.is_chat()).collect()
            } else {
                messages
            };
            if let Some(ref peer) = params.as_peer {
                filtered.retain(|m| crate::transport::ws::should_deliver(m, peer));
            }
            if let Some(ref before_str) = params.before
                && let Ok(before_ts) = before_str.parse::<chrono::DateTime<chrono::Utc>>()
            {
                filtered.retain(|m| m.ts < before_ts);
            }
            filtered.reverse();
            let page: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();
            (
                StatusCode::OK,
                serde_json::to_string(&page).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read: {e}"),
        ),
    }
}

pub async fn post_message(
    State(session): State<Arc<session::Session>>,
    Extension(liveness): Extension<Arc<crate::session::connection_liveness::ConnectionLiveness>>,
    Json(payload): Json<PostMessage>,
) -> impl IntoResponse {
    liveness.seen(payload.from.split(':').next().unwrap_or(&payload.from));
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
    if payload.from.ends_with(":claude-code")
        && let Some(reason) = crate::session::speech_guard::pause_vocab_violation(&payload.text)
    {
        return (StatusCode::BAD_REQUEST, reason);
    }
    let explicit_privacy = payload.to.is_some();
    let msg = session::Message {
        from: payload.from,
        text: payload.text,
        ts: chrono::Utc::now(),
        message_type: session::MessageType::from(payload.message_type.as_deref()),
        to: payload.to,
        addressee: payload.addressee,
        explicit_privacy,
        multi_depth: payload.multi_depth,
        state_delta: None,
        state_snapshot: None,
    };
    if let Err(e) = session.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
}
