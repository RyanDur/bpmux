//! Agent-side E2E test for `move_story` MCP tool (#1188). Drives the
//! full bridge subprocess + canonical-store roundtrip — same harness
//! pattern as label_story_e2e / reorder_story_e2e.
//!
//! Live-bridge integration test per #1188 falsifier-g (live-class
//! requirement; mock-only is the substitution-class per
//! `feedback_test_fidelity_vocabulary_discipline`). Covers:
//!
//! - Happy path: pm moves a story to ready with rationale; tool
//!   returns a message naming the from→to transition (substrate
//!   evidence the chain fired end-to-end through MCP-stdio).
//! - Falsifier-b enforcement: blank rationale rejected.
//! - Falsifier-d enforcement: non-1-based position rejected.
//! - Permission boundary (falsifier-e + scope-question 1):
//!   dev caller setting position in 'ready' is rejected per
//!   PM-organizes-queue-order rule. dev moving COLUMN (no position)
//!   succeeds — column moves are agent-class.
//!
//! What this test does NOT cover (intentional v1 boundary):
//! - Cross-machine canonical-store reconciliation under conflict.
//! - Story-timeline rendering (read-side; covered by view_comments
//!   pickup of the FreeCommentAppended audit-trail).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn move_story_happy_path_returns_message_naming_target_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-happy-target",
            "reject if move_story doesn't fire on the target",
        )
        .await;

        let result = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "rationale": "promote into current iteration"
                }),
            )
            .await
            .expect("move_story to ready with rationale should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("ready"),
            "happy-path message names destination state; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_rejects_blank_rationale_per_falsifier_b() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-blank-rationale-target",
            "reject if blank rationale is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "rationale": "   "
                }),
            )
            .await
            .expect_err("blank rationale must be rejected per #1188 falsifier-b");

        assert!(
            err.contains("rationale"),
            "rejection cites the rationale field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_rejects_zero_position_per_falsifier_d() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-zero-position-target",
            "reject if zero position is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "position": 0,
                    "rationale": "test zero rejection"
                }),
            )
            .await
            .expect_err("zero position must be rejected — position is 1-based");

        assert!(
            err.contains("position") && err.contains("1-based"),
            "rejection cites the 1-based discipline; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_rejects_unknown_target_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-unknown-state-target",
            "reject if unknown target_state is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "bogus",
                    "rationale": "test unknown rejection"
                }),
            )
            .await
            .expect_err("unknown target_state must be rejected with named valid options");

        assert!(
            err.contains("target_state") || err.contains("Valid"),
            "rejection enumerates valid target states; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_dev_caller_with_position_in_ready_rejected_per_pm_lane() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-dev-position-gate-target",
            "reject if dev can specify position in ready bypassing pm-organizes-queue rule",
        )
        .await;

        let err = dev
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "position": 1,
                    "rationale": "dev shouldn't be able to set queue position"
                }),
            )
            .await
            .expect_err(
                "dev caller with position must be rejected per PM-organizes-ready-queue rule",
            );

        assert!(
            err.to_lowercase().contains("pm") || err.contains("position"),
            "rejection cites pm-lane / position discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}
