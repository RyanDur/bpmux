//! Message-channel handlers — REST surface mirroring the WebSocket
//! message-broadcast flow. `get_messages` is the agent-side history
//! reader (chat-only by default, with paged-history + delivery-side
//! filter knobs); `post_message` is the agent-side write endpoint
//! that broadcasts through `session::Session::broadcast`.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::session;

/// Default `limit` for `GET /api/messages` when the caller omits the
/// query parameter. Read by `bridge::tools::read_messages` so the
/// MCP tool and the REST endpoint stay aligned.
pub const DEFAULT_MESSAGE_LIMIT: usize = 20;

/// POST body for `/api/messages`. `to` enables single-recipient
/// privacy semantics (#158 directed delivery), `multi_depth` carries
/// the optional multi-depth rendering payload (#187), and `addressee`
/// is the attention signal (#158).
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

/// Query parameters for `/api/messages`. `chat_only` defaults to
/// true so system/activity classes don't bleed into chat reads;
/// `as_peer` simulates delivery-side filtering for a specific role
/// (#191); `message_type` overrides `chat_only`'s default so callers
/// can hydrate presence without bumping `chat_only=false` (#364);
/// `before` enables paged-history reads (#1153) by RFC3339 timestamp.
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

/// GET /api/messages — page of recent messages, newest-first,
/// filtered per `GetMessagesParams`. The `before` parameter uses
/// strict-less-than (`m.ts < before`) so callers paging older
/// history pass the oldest-currently-rendered timestamp without
/// boundary duplicates; client-side dedup handles any race.
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

/// POST /api/messages — broadcast an agent-sent message through
/// `session::Session::broadcast`. Rejects `message_type="system"`
/// (reserved for internal bridge signals — classifier nudges,
/// handoff warnings, join/leave — which dispatch through
/// `session::log::append` / `emit_system`, never this handler).
/// A non-empty `to` filter sets `explicit_privacy=true` on the
/// stored message so downstream guards (e.g. `/share` refusing to
/// widen private messages) treat HTTP-originated DMs identically
/// to TUI `/to` DMs.
pub async fn post_message(
    State(session): State<Arc<session::Session>>,
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
