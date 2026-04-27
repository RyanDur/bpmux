//! HTTP client for cross-process tool dispatch from the bridge subcommand
//! to electron-server's canonical SyncOrchestrator (#516).
//!
//! Slice 2: bridges stop running mutation handlers against their own
//! local BoardStore + adapter and instead POST to
//! `/api/rpc/<tool>` on `http://127.0.0.1:{port}` (the electron-server
//! they're already WS-connected to). Slice 1 added the receiving
//! endpoint; this module is the calling side.
//!
//! Auth-token discovery: same source the electron-server's `auth::gate`
//! middleware verifies against — `BPMUX_AUTH_TOKEN_PATH` override, else
//! `default_auth_token_path()` (`~/.bpmux/auth_token`). Bridge + server
//! share a filesystem so they read the same token by construction.
//!
//! Failure mode is fail-loud per #516 designer amendment 1: if the
//! electron-server is offline / unreachable / returns a non-2xx, the
//! caller gets a structured error string naming the cause. Bridges do
//! NOT silently fall back to local-handler dispatch — that would
//! resurrect the dual-store divergence #516 exists to eliminate.

use serde_json::{Map, Value};

const RPC_AGENT_HEADER: &str = "X-BPMUX-Agent";

/// Resolve the per-run auth token the bridge presents to the electron-
/// server. Mirrors the discovery path `auth::gate` validates against on
/// the receiving side: BPMUX_AUTH_TOKEN_PATH env override else
/// `~/.bpmux/auth_token`. Bridge + server share a filesystem so they
/// observe the same token by construction.
fn load_auth_token() -> Result<String, String> {
    let path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    crate::session::auth_token::read(&path)
        .map_err(|e| format!("read auth-token from {}: {e}", path.display()))
}

/// POST a JSON body to `/api/rpc/<tool>` on the local electron-server,
/// authenticated as `agent`. Returns the server's `message` field on
/// success, or a structured error otherwise. The error string names
/// the cause (unreachable, non-2xx status, malformed body) so the
/// caller can surface it through MCP's error channel without
/// guessing.
async fn post_rpc(
    port: u16,
    tool: &str,
    agent: &str,
    body: &Value,
    token: &str,
) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/api/rpc/{tool}");
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .header(RPC_AGENT_HEADER, agent)
        .json(body)
        .send()
        .await
        .map_err(|e| {
            format!("rpc {tool}: electron-server unreachable at {url}: {e} — bridge requires the canonical store; ensure Electron is running")
        })?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status.is_success() {
        parsed
            .get("message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| format!("rpc {tool}: success status {status} but response missing 'message' field: {text}"))
    } else {
        let err = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or(text.as_str())
            .to_string();
        Err(format!("rpc {tool}: {status} — {err}"))
    }
}

/// Bridge-side dispatch for `start_story`: forwards the MCP args to
/// the electron-server's canonical SyncOrchestrator. Replaces
/// `tools_stories::handle_start_story` at the bridge dispatch site
/// (#516 slice 2 archetype-validation).
pub(super) async fn start_story_via_rpc(
    port: u16,
    agent: &str,
    args: &Map<String, Value>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "missing 'number' argument".to_string())?;
    let pickup_paraphrase = args
        .get("pickup_paraphrase")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'pickup_paraphrase' argument".to_string())?;
    let falsifier_reframing = args
        .get("falsifier_reframing")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'falsifier_reframing' argument".to_string())?;
    let body = serde_json::json!({
        "number": number,
        "pickup_paraphrase": pickup_paraphrase,
        "falsifier_reframing": falsifier_reframing,
    });
    let token = load_auth_token()?;
    post_rpc(port, "start_story", agent, &body, &token).await
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use httpmock::{Method, MockServer};

    fn body(number: u64) -> Value {
        serde_json::json!({
            "number": number,
            "pickup_paraphrase": "paraphrase",
            "falsifier_reframing": "falsifier",
        })
    }

    #[tokio::test]
    async fn post_rpc_carries_bearer_and_agent_header_to_endpoint() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(Method::POST)
                .path("/api/rpc/start_story")
                .header("authorization", "Bearer test-token-123")
                .header(RPC_AGENT_HEADER, "dev")
                .body_includes("\"number\":42")
                .body_includes("\"pickup_paraphrase\":\"paraphrase\"");
            then.status(200)
                .header("content-type", "application/json")
                .body(r##"{"message":"#42 marked as in progress"}"##);
        });
        let port: u16 = server.port();

        let result = post_rpc(port, "start_story", "dev", &body(42), "test-token-123").await;

        m.assert();
        assert_eq!(result, Ok("#42 marked as in progress".to_string()));
    }

    #[tokio::test]
    async fn post_rpc_surfaces_named_error_on_offline_server() {
        // Port 1 is reserved + nothing's listening — the connect should
        // fail at the TCP layer + we surface the "unreachable" framing.
        let result = post_rpc(1, "start_story", "dev", &body(42), "tk").await;
        let err = result.expect_err("offline server must fail-loud");
        assert!(
            err.contains("electron-server unreachable"),
            "expected named cause in {err}"
        );
    }

    #[tokio::test]
    async fn post_rpc_surfaces_server_error_body_on_4xx() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(Method::POST).path("/api/rpc/start_story");
            then.status(400)
                .header("content-type", "application/json")
                .body(r#"{"error":"only dev role can start_story"}"#);
        });
        let port: u16 = server.port();

        let result = post_rpc(port, "start_story", "pm", &body(42), "tk").await;

        m.assert();
        let err = result.expect_err("4xx must surface as Err");
        assert!(
            err.contains("only dev role can start_story"),
            "expected server-error message in {err}"
        );
    }

    #[tokio::test]
    async fn post_rpc_surfaces_missing_message_field_on_2xx() {
        let server = MockServer::start();
        let m = server.mock(|when, then| {
            when.method(Method::POST).path("/api/rpc/start_story");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"unexpected":"shape"}"#);
        });
        let port: u16 = server.port();

        let result = post_rpc(port, "start_story", "dev", &body(42), "tk").await;
        m.assert();
        let err = result.expect_err("missing message field must surface");
        assert!(
            err.contains("missing 'message' field"),
            "expected protocol-error framing in {err}"
        );
    }

    #[test]
    fn start_story_via_rpc_validates_required_args() {
        // Argument-validation fires before the HTTP round-trip — verify
        // that missing fields produce a structured error without us
        // needing to spin a mock server.
        let mut args = Map::new();
        args.insert("number".to_string(), Value::Number(1.into()));
        // Missing pickup_paraphrase + falsifier_reframing
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("rt");
        let err = rt
            .block_on(async { start_story_via_rpc(0, "dev", &args).await })
            .expect_err("missing required args");
        assert!(
            err.contains("pickup_paraphrase"),
            "expected pickup_paraphrase in error, got {err}"
        );
    }
}
