//! Agent-side E2E test for `set_status` MCP tool (#476 Phase 1
//! backfill — signal-broadcast variant of bridge-state-mutation).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — set_status applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! set_status broadcasts a status-message via WebSocket with
//! `message_type: activity`. Cross-bridge propagation is filtered
//! (activity-class messages may not route to other roles' notification
//! channels by default) — propagation falsifier deferred until
//! filter-semantics audit lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn set_status_returns_confirmation_with_status_text() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "set_status",
                serde_json::json!({"status": "STATUS_MARKER_SET_S1"}),
            )
            .await
            .expect("set_status should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("status set to"),
            "happy-path message confirms status-set; got {msg}"
        );
        assert!(
            msg.contains("STATUS_MARKER_SET_S1"),
            "message echoes the status text verbatim; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn set_status_rejects_missing_status() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("set_status", serde_json::json!({}))
            .await
            .expect_err("missing status should be rejected");

        assert!(
            err.contains("status"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
