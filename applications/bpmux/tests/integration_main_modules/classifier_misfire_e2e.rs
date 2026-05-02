//! Agent-side E2E test for `classifier_misfire` MCP tool (#476 Phase 1
//! backfill — feedback-capture variant). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — classifier_misfire applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn classifier_misfire_with_reason_returns_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "classifier_misfire",
                serde_json::json!({
                    "nudge_hash": "test-nudge-hash-1",
                    "reason": "false-positive on routine ack"
                }),
            )
            .await
            .expect("classifier_misfire should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("misfire recorded"),
            "happy-path message confirms misfire-capture; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn classifier_misfire_without_reason_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "classifier_misfire",
                serde_json::json!({"nudge_hash": "test-nudge-hash-2"}),
            )
            .await
            .expect("classifier_misfire without reason should succeed (Naur framing)");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("misfire recorded"),
            "no-reason path also confirms; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn classifier_misfire_rejects_missing_nudge_hash() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("classifier_misfire", serde_json::json!({}))
            .await
            .expect_err("missing nudge_hash should be rejected");

        assert!(
            err.contains("nudge_hash"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
