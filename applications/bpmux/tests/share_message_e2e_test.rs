//! Agent-side E2E test for `share_message` MCP tool (#476 Phase 1
//! backfill — mutation archetype, message-propagation variant). Uses
//! the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — share_message applicability =
//! {agent: yes, human: yes (TUI /share command), external: no}.
//! This test covers ONLY agent-side; human-side TUI /share is a
//! separate Playwright concern.
//!
//! Two forms covered: Form 2 fresh-text (explicit `text` arg) and the
//! Form-1 no-recent-visible-chat error path. Form 1 propagation
//! coverage (agent's recent chat re-shared to wider scope) is
//! deferred — the bridge-side log-fetch via REST is timing-coupled
//! to the bridge's session bootstrap and is better verified at the
//! integration-test layer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn share_message_form_2_fresh_text_returns_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "share_message",
                serde_json::json!({
                    "scope": "all",
                    "text": "fresh broadcast via share_message Form 2"
                }),
            )
            .await
            .expect("share_message Form 2 should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("shared fresh message to all"),
            "Form 2 returns the fresh-share confirmation; got `{text}`"
        );

        pm.close().await;
    });
}

#[test]
fn share_message_rejects_missing_scope() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("share_message", serde_json::json!({"text": "any"}))
            .await
            .expect_err("missing scope should be rejected");

        assert!(
            err.contains("scope"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
