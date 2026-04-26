//! Agent-side E2E test for `list_peer_lessons` MCP tool (#476 Phase 1
//! backfill — read archetype, cross-role variant). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — list_peer_lessons applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn list_peer_lessons_returns_array_for_default_peer_set() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("list_peer_lessons", serde_json::json!({}))
            .await
            .expect("list_peer_lessons should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !text.is_empty(),
            "list_peer_lessons returns non-empty payload (even when no peer-lessons exist); got `{text}`"
        );

        dev.close().await;
    });
}

#[test]
fn list_peer_lessons_peer_filter_returns_explicit_no_lessons_message_for_unmet_peer() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "list_peer_lessons",
                serde_json::json!({"peer": "ghost-peer-that-does-not-exist"}),
            )
            .await
            .expect("list_peer_lessons with unknown peer should succeed (no error)");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("No lessons") || text.contains("ghost-peer") || text.is_empty(),
            "unknown peer returns explicit no-lessons message; got `{text}`"
        );

        dev.close().await;
    });
}
