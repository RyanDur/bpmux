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
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind cites workflow-role-not-allowed; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"reorder\""),
            "rejection cites attempted_transition; got {err}"
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
