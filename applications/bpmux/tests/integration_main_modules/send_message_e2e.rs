//! Agent-side E2E test for `send_message` MCP tool (#476 Phase 1
//! backfill — broadcast variant). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — send_message applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn send_message_propagates_to_peer_bridge() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "SEND_PROPAGATION_MARKER_M2";
        dev.call_tool("send_message", serde_json::json!({"text": marker}))
            .await
            .expect("send_message should succeed");

        pm.capture.wait_for(marker, Duration::from_secs(5)).await;

        dev.close().await;
        pm.close().await;
    });
}

#[test]
fn send_message_rejects_missing_text() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("send_message", serde_json::json!({}))
            .await
            .expect_err("missing text should be rejected");

        assert!(
            err.contains("text"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
