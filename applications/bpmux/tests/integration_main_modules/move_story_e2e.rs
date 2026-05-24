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

/// Falsifier-c (LOAD-BEARING — the bug-class this story was filed
/// to fix per spec body's '#1177 stuck-in-Accepted is exactly this').
/// Drives the closed-story → non-terminal recovery path end-to-end:
/// create → accept (closes the GH issue + transitions to Accepted) →
/// move_story back to ready with rationale → assert success. The
/// substrate fires StoryReopened first then StateTransitioned, so a
/// successful tool return + non-error message is operational evidence
/// the recovery cascade works.
#[test]
fn move_story_recovers_closed_story_back_to_ready_per_falsifier_c() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-recover-from-accepted-target",
            "reject if move_story can't un-stick a closed/Accepted story",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "number": target_number,
                "target_state": "accepted",
                "rationale": "stage close so the recovery path has something to undo"
            }),
        )
        .await
        .expect("move_story to accepted should succeed (closes issue + transitions Accepted)");

        let recovery = pm
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "rationale": "recover from wrong-Accept — the #1177-stuck class fix"
                }),
            )
            .await
            .expect(
                "move_story from accepted → ready must succeed (StoryReopened-then-StateTransitioned cascade per falsifier-c — this is the bug-class this story was filed to fix)",
            );

        let msg = recovery.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")) && msg.contains("ready"),
            "recovery message names target story + destination state; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn move_story_dev_bare_column_move_is_pm_rejected_1548() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-dev-column-only-target",
            "reject if dev bare column-move is not PM-gated (#1548 supersedes Ryan-01:54 agent-class column-move with the Ryan-2026-05-24 role-reset: the board is the PM's lane)",
        )
        .await;

        let err = dev
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "icebox",
                    "rationale": "dev bare column-move must be role-rejected per #1548 PM-only-on-all-paths"
                }),
            )
            .await
            .expect_err(
                "dev bare column-move must 409 — move_story is PM-only on ALL paths (#1548, Ryan 2026-05-24 role-reset)",
            );

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\"") && err.contains("\"actor_role\":\"dev\""),
            "dev bare column-move rejection cites workflow-role-not-allowed for actor dev; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

/// Falsifier-f end-to-end: audit-trail comment is visible after the
/// move via view_comments. Unit tests cover the body shape; this
/// test exercises the FreeCommentAppended → canonical-store →
/// view_comments-readback cascade so the operator-visibility
/// promise lands.
#[test]
fn move_story_audit_comment_surfaces_via_view_comments_per_falsifier_f() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] move-story-audit-trail-target",
            "reject if move_story doesn't surface audit-comment via view_comments",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "number": target_number,
                "target_state": "ready",
                "rationale": "AUDIT_TRAIL_MARKER_RATIONALE for view_comments readback"
            }),
        )
        .await
        .expect("move_story should succeed");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let comments = pm
            .call_tool(
                "view_comments",
                serde_json::json!({ "number": target_number }),
            )
            .await
            .expect("view_comments should succeed");

        let dump = serde_json::to_string(&comments).unwrap_or_default();
        assert!(
            dump.contains("AUDIT_TRAIL_MARKER_RATIONALE"),
            "view_comments must surface the rationale from the move_story audit-comment per falsifier-f; got {dump}"
        );
        assert!(
            dump.contains("move_story") && dump.contains("ready"),
            "audit-comment body must name the action + destination state; got {dump}"
        );

        pm.close().await;
    });
}
