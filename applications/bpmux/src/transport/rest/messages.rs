use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::hub;
use crate::routing::short_name;

use super::shared::OWNER_NONCE_HEADER;

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
    #[serde(default)]
    pub sender_id: Option<String>,
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
    axum::Extension(select): axum::Extension<crate::store::Select>,
    headers: HeaderMap,
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
    let resolved_ids: Vec<String> = as_peer
        .as_deref()
        .map(|peer| select.select(|s| crate::store::routing::resolve(&s.routing, short_name(peer))))
        .unwrap_or_default();
    let claimed_owner_nonce =
        select.select(|s| crate::store::owner_session::claimed_nonce(&s.owner_session));
    let presented_owner_nonce = headers
        .get(OWNER_NONCE_HEADER)
        .and_then(|value| value.to_str().ok());
    let is_owner = as_peer.as_deref().is_some_and(|peer| {
        crate::store::owner_session::resolve_owner_mark(
            claimed_owner_nonce.as_deref(),
            presented_owner_nonce,
            peer,
        )
    });
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
        if let Some(ref peer) = as_peer {
            let reaches = if resolved_ids.is_empty() {
                crate::transport::ws::should_deliver(m, None, peer, is_owner)
            } else {
                resolved_ids
                    .iter()
                    .any(|id| crate::transport::ws::should_deliver(m, Some(id), peer, is_owner))
            };
            if !reaches {
                return false;
            }
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
    axum::Extension(select): axum::Extension<crate::store::Select>,
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
    let mut msg = crate::message::Message {
        id: String::new(),
        from: payload.from,
        sender_id: payload.sender_id,
        text: payload.text,
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::from(payload.message_type.as_deref()),
        to: payload.to,
        to_ids: None,
        addressee: payload.addressee,
        addressee_ids: None,
        explicit_privacy,
        multi_depth: payload.multi_depth,
        state_delta: None,
        state_snapshot: None,
    };
    crate::transport::delivery_ids::stamp_delivery_ids(&select, &mut msg);
    if let Err(e) = hub.broadcast(&msg) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to broadcast: {e}"),
        );
    }
    (StatusCode::CREATED, "message sent".to_string())
}
