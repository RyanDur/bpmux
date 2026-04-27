//! RPC routes for cross-process bridge → electron-server tool dispatch (#516).
//!
//! Architectural-shift slices:
//! - Slice 1: protocol shape + one canonical mutation endpoint.
//! - Slice 2: bridge `start_story` migrated through it (archetype-validation).
//! - Slice 3: generalize to `POST /api/rpc/{tool}`; route every mutation
//!   tool through the same endpoint per template-replication
//!   (`feedback_archetype_validation_unlocks_template_replication`).
//! - Slice 4: remove the bridge-side BoardStore + SyncOrchestrator so
//!   electron-server is the sole canonical store.
//!
//! Auth model: routes sit behind the existing Bearer + loopback gate
//! (same as `/api/simulate/*`). The caller's role identity flows in via
//! the `X-BPMUX-Agent` header — handlers like `handle_start_story` are
//! role-gated (`require_role`) and a missing or unknown role would
//! short-circuit there. The header is the lightest-weight way to
//! express "who is invoking" without restructuring the auth state.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::Value;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;
use crate::session::{self, Message, MessageType};

pub const AGENT_HEADER: &str = "X-BPMUX-Agent";

/// Roles the bridge can identify as. The MCP handlers' `require_role`
/// gate enforces tool-by-tool admission; this constant is the
/// envelope-level liveness check before we even get there.
const KNOWN_AGENTS: &[&str] = &["dev", "pm", "designer", "owner"];

/// Tools that the generic RPC endpoint dispatches. Each name maps to a
/// `bridge::dispatch_test_<tool>` wrapper. Read-only tools (view_board,
/// get_story, etc.) are not yet covered — they still read from the
/// bridge's own BoardStore today and are migrated in slice 4 alongside
/// the canonical-store cutover.
fn known_mutation_tools() -> &'static [&'static str] {
    &[
        "start_story",
        "deliver_story",
        "pass_desk_check",
        "fail_desk_check",
        "accept_story",
        "reject_story",
        "close_story",
        "create_story",
        "comment_on_story",
        "label_story",
        "set_parent_epic",
        "unset_parent_epic",
        "reorder_story",
    ]
}

/// Resolve the caller role from the `X-BPMUX-Agent` request header.
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
/// RPC dispatch path takes an empty WsSink, so we restore the live-
/// update broadcast at the route boundary so connected clients refetch
/// /api/board.
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

/// Coerce the JSON-Object body into the `Map<String, Value>` shape the
/// handler dispatchers expect. RPC clients always send a JSON object;
/// any other shape is a protocol error.
fn args_from_value(body: Value) -> Result<serde_json::Map<String, Value>, String> {
    match body {
        Value::Object(map) => Ok(map),
        other => Err(format!(
            "RPC body must be a JSON object; got {}",
            match other {
                Value::Null => "null",
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => unreachable!(),
            }
        )),
    }
}

/// Generic dispatch — the body is the MCP-tool args as JSON; the path
/// parameter names the tool. Slice 3 routes every mutation tool through
/// here so the bridge's per-tool dispatch sites collapse to one-line
/// `dispatch_via_rpc(...)` calls.
async fn dispatch(
    refresh: &Arc<dyn RefreshDriver>,
    tool: &str,
    agent: &str,
    args: serde_json::Map<String, Value>,
) -> Result<String, String> {
    let refresh_ref: Option<&Arc<dyn RefreshDriver>> = Some(refresh);
    match tool {
        "start_story" => bridge::dispatch_test_start_story(args, agent, refresh_ref).await,
        "deliver_story" => bridge::dispatch_test_deliver_story(args, agent, refresh_ref).await,
        "pass_desk_check" => bridge::dispatch_test_pass_desk_check(args, agent, refresh_ref).await,
        "fail_desk_check" => bridge::dispatch_test_fail_desk_check(args, agent, refresh_ref).await,
        "accept_story" => bridge::dispatch_test_accept_story(args, agent, refresh_ref).await,
        "reject_story" => bridge::dispatch_test_reject_story(args, agent, refresh_ref).await,
        "close_story" => bridge::dispatch_test_close_story(args, agent, refresh_ref).await,
        "create_story" => bridge::dispatch_test_create_story(args, agent, refresh_ref).await,
        "comment_on_story" => {
            bridge::dispatch_test_comment_on_story(args, agent, refresh_ref).await
        }
        "label_story" => bridge::dispatch_test_label_story(args, agent, refresh_ref).await,
        "set_parent_epic" => bridge::dispatch_test_set_parent_epic(args, agent, refresh_ref).await,
        "unset_parent_epic" => {
            bridge::dispatch_test_unset_parent_epic(args, agent, refresh_ref).await
        }
        "reorder_story" => bridge::dispatch_test_reorder_story(args, agent).await,
        _ => Err(format!(
            "rpc: unknown tool '{tool}'; expected one of {:?}",
            known_mutation_tools()
        )),
    }
}

/// `POST /api/rpc/{tool}` — generic dispatch endpoint backing all
/// mutation tools the bridge forwards (#516 slices 1-3). Path param
/// names the tool; body is the MCP args object.
pub async fn post_rpc_tool(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(body): Json<Value>,
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
    let args = match args_from_value(body) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    match dispatch(&refresh, &tool, &agent, args).await {
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

    #[test]
    fn args_from_value_accepts_object() {
        let body = serde_json::json!({"a": 1, "b": "two"});
        let args = args_from_value(body).unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn args_from_value_rejects_non_object() {
        let body = serde_json::json!([1, 2]);
        let err = args_from_value(body).unwrap_err();
        assert!(err.contains("must be a JSON object"));
    }

    #[test]
    fn known_mutation_tools_covers_full_state_machine() {
        let tools = known_mutation_tools();
        for required in [
            "start_story",
            "deliver_story",
            "pass_desk_check",
            "fail_desk_check",
            "accept_story",
            "reject_story",
            "close_story",
        ] {
            assert!(
                tools.contains(&required),
                "{required} must be in known_mutation_tools (load-bearing for state-cascade)"
            );
        }
    }
}
