//! Agent-side E2E tests for simulate_* MCP tools (#476 Phase 1
//! backfill — UI-input-simulation archetype, agent-driving-TUI variant).
//! Uses the existing `tests/harness/bridge.rs` substrate + the
//! `auth_token_path` extension from #487.
//!
//! Per the 3-class framing — simulate_* applicability =
//! {agent: yes, human: no (simulates the human; humans don't INVOKE),
//! external: no}. Single-class agent-only.
//!
//! Bridge subprocess needs `BPMUX_AUTH_TOKEN_PATH` pointing at the
//! bpmux server's auth_token file so its `load_auth_token()` reads
//! the SAME token the server wrote (instead of the developer's real
//! `~/.bpmux/auth_token`). This closes the auth-token-cross-process
//! gap surfaced during Phase 1 backfill.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn simulate_click_via_mcp_returns_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let token_path = bpmux.auth_token_path();
        let token_path_str = token_path.to_str().expect("utf-8 path");
        let dev = TestBridge::connect_with_env(
            &bpmux,
            "dev",
            &[("BPMUX_AUTH_TOKEN_PATH", token_path_str)],
        )
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("simulate_click", serde_json::json!({"x": 5, "y": 0}))
            .await
            .expect("simulate_click via MCP should succeed when auth-token-path shared");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !text.is_empty(),
            "simulate_click returns non-empty payload from /api/simulate/click; got `{text}`"
        );

        dev.close().await;
    });
}

#[test]
fn simulate_click_rejects_missing_x() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let token_path = bpmux.auth_token_path();
        let token_path_str = token_path.to_str().expect("utf-8 path");
        let dev = TestBridge::connect_with_env(
            &bpmux,
            "dev",
            &[("BPMUX_AUTH_TOKEN_PATH", token_path_str)],
        )
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("simulate_click", serde_json::json!({"y": 0}))
            .await
            .expect_err("missing x should be rejected");

        assert!(
            err.contains("x"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn simulate_input_submit_via_mcp_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let token_path = bpmux.auth_token_path();
        let token_path_str = token_path.to_str().expect("utf-8 path");
        let dev = TestBridge::connect_with_env(
            &bpmux,
            "dev",
            &[("BPMUX_AUTH_TOKEN_PATH", token_path_str)],
        )
        .await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("simulate_input_submit", serde_json::json!({}))
            .await
            .expect("simulate_input_submit via MCP should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !text.is_empty(),
            "simulate_input_submit returns non-empty payload; got `{text}`"
        );

        dev.close().await;
    });
}
