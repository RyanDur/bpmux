//! RPC routes for cross-process bridge → electron-server tool dispatch (#516).
//!
//! Slice 1: protocol shape + one canonical mutation endpoint
//! (`POST /api/rpc/start_story`). Additive — no bridge behavior change yet.
//! Subsequent slices migrate bridge tools to call into these endpoints
//! (slice 2: start_story; slice 3: remaining mutation + read tools;
//! slice 4: remove the bridge-side BoardStore + SyncOrchestrator so
//! electron-server is the sole canonical store).
//!
//! Auth model: routes sit behind the existing Bearer + loopback gate
//! (same as `/api/simulate/*`). The caller's role identity flows in via
//! the `X-BPMUX-Agent` header — handlers like `handle_start_story` are
//! role-gated (`require_role`) and a missing or unknown role would
//! short-circuit there. The header is the lightest-weight way to
//! express "who is invoking" without restructuring the auth state.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;
use crate::session::{self, Message, MessageType};

pub const AGENT_HEADER: &str = "X-BPMUX-Agent";

/// Roles the bridge can identify as. The MCP handlers' `require_role`
/// gate enforces tool-by-tool admission; this constant is the
/// envelope-level liveness check before we even get there. Keep in
/// sync with the role set the handlers expect.
const KNOWN_AGENTS: &[&str] = &["dev", "pm", "designer", "owner"];

/// Resolve the caller role from the `X-BPMUX-Agent` request header.
/// Returns the canonical lowercase role string on success, or a
/// human-readable reason on failure (used as the `error` body so the
/// caller knows whether to fix its env or its request).
pub fn agent_from_headers(headers: &HeaderMap) -> Result<String, String> {
    let raw = headers
        .get(AGENT_HEADER)
        .ok_or_else(|| format!("{AGENT_HEADER} header required"))?
        .to_str()
        .map_err(|_| format!("{AGENT_HEADER} header is not valid UTF-8"))?
        .trim()
        .to_ascii_lowercase();
    if raw.is_empty() {
        return Err(format!("{AGENT_HEADER} header is empty"));
    }
    if !KNOWN_AGENTS.contains(&raw.as_str()) {
        return Err(format!(
            "{AGENT_HEADER} '{raw}' is not a known role; expected one of {KNOWN_AGENTS:?}"
        ));
    }
    Ok(raw)
}

/// Mirror of `transport::test_routes::fire_board_invalidate` (#490). The
/// RPC dispatch path takes an empty WsSink (same shape as test-route
/// dispatch), so we restore the live-update broadcast at the route
/// boundary so connected clients refetch /api/board.
fn fire_board_invalidate(session: &Arc<session::Session>) {
    let msg = Message {
        from: "bpmux:rpc".to_string(),
        text: crate::bridge::BOARD_INVALIDATE_MARKER.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::session::diag::log_diag(&format!("rpc: BOARD_INVALIDATE broadcast failed: {e}"));
    }
}

#[derive(Debug, Deserialize)]
pub struct StartStoryReq {
    pub number: u64,
    pub pickup_paraphrase: String,
    pub falsifier_reframing: String,
}

/// `POST /api/rpc/start_story` — canonical-store dispatch for the dev
/// `start_story` MCP tool. Slice 1 endpoint; bridge will be migrated to
/// call this in slice 2 instead of running the handler against its own
/// local SyncOrchestrator.
pub async fn post_rpc_start_story(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    headers: HeaderMap,
    Json(req): Json<StartStoryReq>,
) -> impl IntoResponse {
    let agent = match agent_from_headers(&headers) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "canonical store not configured (no GitHub orchestrator built at startup)"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert(
        "pickup_paraphrase".to_string(),
        serde_json::Value::String(req.pickup_paraphrase),
    );
    args.insert(
        "falsifier_reframing".to_string(),
        serde_json::Value::String(req.falsifier_reframing),
    );
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_start_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => {
            fire_board_invalidate(&session);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn header_map(values: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (k, v) in values {
            headers.insert(*k, HeaderValue::from_str(v).expect("test header"));
        }
        headers
    }

    #[test]
    fn agent_from_headers_accepts_known_role_lowercase() {
        let h = header_map(&[(AGENT_HEADER, "dev")]);
        assert_eq!(agent_from_headers(&h).unwrap(), "dev");
    }

    #[test]
    fn agent_from_headers_canonicalizes_case_and_whitespace() {
        let h = header_map(&[(AGENT_HEADER, "  PM  ")]);
        assert_eq!(agent_from_headers(&h).unwrap(), "pm");
    }

    #[test]
    fn agent_from_headers_rejects_missing_header() {
        let err = agent_from_headers(&HeaderMap::new()).unwrap_err();
        assert!(
            err.contains("required"),
            "expected 'required' in {err}, got {err:?}"
        );
    }

    #[test]
    fn agent_from_headers_rejects_unknown_role() {
        let h = header_map(&[(AGENT_HEADER, "intern")]);
        let err = agent_from_headers(&h).unwrap_err();
        assert!(
            err.contains("not a known role"),
            "expected 'not a known role' in {err}, got {err:?}"
        );
    }

    #[test]
    fn agent_from_headers_rejects_empty_value() {
        let h = header_map(&[(AGENT_HEADER, "   ")]);
        let err = agent_from_headers(&h).unwrap_err();
        assert!(
            err.contains("empty"),
            "expected 'empty' in {err}, got {err:?}"
        );
    }
}
