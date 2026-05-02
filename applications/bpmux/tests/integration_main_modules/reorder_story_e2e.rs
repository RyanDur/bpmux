//! Agent-side E2E test for `reorder_story` MCP tool (#476 Phase 1
//! backfill — mutation archetype, priority-index variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — reorder_story applicability =
//! {agent: yes, human: yes (drag-reorder Electron surface),
//! external: no}. This test covers ONLY agent-side; human-side
//! follows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn reorder_story_returns_message_naming_position_and_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "number": 100,
                    "position": 0,
                    "state": "ready"
                }),
            )
            .await
            .expect("reorder_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("#100"),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("position 0"),
            "message names new position; got {msg}"
        );
        assert!(
            msg.contains("ready"),
            "message names target state; got {msg}"
        );
    });
}

#[test]
fn reorder_story_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "number": 100,
                    "position": 0,
                    "state": "ready"
                }),
            )
            .await
            .expect_err("dev should be rejected by reorder pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn reorder_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "reorder_story",
                serde_json::json!({"position": 0, "state": "ready"}),
            )
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn reorder_story_rejects_missing_position() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "reorder_story",
                serde_json::json!({"number": 100, "state": "ready"}),
            )
            .await
            .expect_err("missing position should be rejected");

        assert!(
            err.contains("position"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn reorder_story_rejects_missing_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "reorder_story",
                serde_json::json!({"number": 100, "position": 0}),
            )
            .await
            .expect_err("missing state should be rejected");

        assert!(
            err.contains("state"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
