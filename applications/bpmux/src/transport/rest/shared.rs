//! Shared helpers + types used across rest feature submodules.
//!
//! `TrackerHandle` is the per-request optional GitHub adapter — None
//! when the operator runs bpmux without a tracker; handlers that
//! require one return 503 so the renderer surfaces "tracker not
//! configured" instead of a silent empty board. `err_body` /
//! `json_headers` shape every REST response uniformly so the renderer
//! can branch on the `error` key without string-matching response
//! text. `broadcast_board_invalidate` mirrors the MCP bridge's
//! same-named function so REST mutations and MCP-tool mutations both
//! drive a TUI refresh through one marker.

use std::sync::Arc;

use serde::Deserialize;

use crate::session;

/// Per-request GitHub-source-adapter handle. None when tracker is
/// unconfigured; handlers then return 503 so the renderer can surface
/// "tracker not configured" instead of a silent empty board.
pub type TrackerHandle = Option<Arc<crate::board::adapters::github::GitHubSourceAdapter>>;

/// JSON body shape for comment-mutation endpoints (POST
/// /api/stories/:n/comments, accept/reject paths).
#[derive(Deserialize)]
pub struct CommentPayload {
    pub body: String,
}

/// Structured error body. All handlers reply with this shape on
/// failure so the renderer can branch on the `error` key without
/// string-matching response text.
pub(super) fn err_body(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

pub(crate) fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}

/// Broadcast a board-invalidate marker through the session. Mirrors
/// the MCP bridge's `broadcast_board_invalidate` — same marker text,
/// same system message_type — so the TUI's event loop picks it up
/// and triggers an immediate board refetch regardless of which path
/// (MCP tool or HTTP REST) caused the mutation.
pub(super) fn broadcast_board_invalidate(session: &session::Session) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: crate::bridge::BOARD_INVALIDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::System,
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("rest: board-invalidate broadcast failed: {e}"));
    }
}
