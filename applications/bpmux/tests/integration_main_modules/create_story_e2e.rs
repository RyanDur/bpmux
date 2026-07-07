//! Agent-side E2E test for `create_story` MCP tool (#476 Phase 1
//! backfill). Uses the existing `tests/harness/bridge.rs` substrate:
//! spawns a real `bpmux bridge --fake-tracker` subprocess, connects
//! via real MCP-stdio (rmcp `TokioChildProcess`), invokes the MCP
//! tool through real protocol, asserts on parsed response.
//!
//! This is **agent-user E2E** per the 3-class framing — full path
//! agent → MCP-stdio → bridge subprocess → real handle_create_story
//! → SyncOrchestrator → InMemorySourceAdapter (via --fake-tracker)
//! → BoardStore. No in-process axum router shortcut.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn consecutive_creates_yield_distinct_stories() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let first = create_target_story(
            &pm,
            "[Feature] first-of-two",
            "reject if a second create silently lands on the first story's number",
        )
        .await;
        let second = create_target_story(
            &pm,
            "[Feature] second-of-two",
            "reject if a second create silently lands on the first story's number",
        )
        .await;
        assert_ne!(
            first, second,
            "each create allocates its own number — colliding numbers silently merge two stories"
        );

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view.as_str().unwrap_or("").to_string();
        assert!(
            view_text.contains("first-of-two") && view_text.contains("second-of-two"),
            "both created stories exist side by side on the board; got:\n{view_text}"
        );

        pm.close().await;
    });
}

#[test]
fn create_story_pm_returns_assigned_number_in_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] e2e-create-story-marker",
                    "user_job": "When testing agent-side E2E, dev needs the create_story MCP tool to round-trip via real subprocess.",
                    "observation_falsifier": "reject if response message omits the GH-assigned number"
                }),
            )
            .await
            .expect("PM should be allowed to call create_story");

        let msg = result
            .as_str()
            .or_else(|| result["message"].as_str())
            .expect("create_story returns a message");
        assert!(
            msg.contains("story created:"),
            "happy-path message names creation; got {msg}"
        );
        assert!(
            msg.contains('#'),
            "message includes the GH-assigned number; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn create_story_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] dev-cannot-file",
                    "user_job": "When testing the lane-gate, dev needs the call to be rejected.",
                    "observation_falsifier": "reject if dev can file a story bypassing pm-lane gate"
                }),
            )
            .await
            .expect_err("dev should be rejected by create_story pm-lane gate");

        assert!(
            err.contains("pm-lane"),
            "rejection cites pm-lane discipline per `feedback_dev_does_not_file_stories`; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn create_story_rejects_missing_observation_falsifier() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] missing-falsifier",
                    "user_job": "When testing required-field validation, the call must surface the missing field."
                }),
            )
            .await
            .expect_err("missing observation_falsifier should be rejected");

        assert!(
            err.contains("observation_falsifier"),
            "rejection names the missing field per filing-discipline; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn create_story_view_board_reflects_new_story_post_create() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        pm.call_tool(
            "create_story",
            serde_json::json!({
                "title": "[Feature] e2e-roundtrip-view-board",
                "user_job": "When testing roundtrip, agent calls create_story then view_board to confirm propagation.",
                "observation_falsifier": "reject if view_board doesn't reflect the newly-created story"
            }),
        )
        .await
        .expect("create_story should succeed");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view.as_str().unwrap_or("").to_string();
        assert!(
            view_text.contains("e2e-roundtrip-view-board"),
            "view_board reflects the new story title via real MCP roundtrip; got:\n{view_text}"
        );

        pm.close().await;
    });
}
