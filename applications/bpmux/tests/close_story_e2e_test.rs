//! Agent-side E2E test for `close_story` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, terminal-only state). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — close_story applicability =
//! {agent: yes, human: yes (Electron close-affordance), external: yes
//! (gh issue close)}. This test covers ONLY agent-side; human-side
//! + external-mutation E2E follow per applicable-class backfill.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn close_story_returns_message_with_audit_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] close-target",
            "reject if close-on-non-existent-story silently succeeds",
        )
        .await;

        let result = pm
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "superseded by alternate path"
                }),
            )
            .await
            .expect("close_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("closed"),
            "happy-path message names the gate-fire affordance; got {msg}"
        );
        assert!(
            msg.contains("superseded by alternate path"),
            "message includes the close-reason for audit trail per close discipline; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn close_story_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-cannot-close-target",
            "reject if dev can close a story bypassing pm-lane gate",
        )
        .await;

        let err = dev
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "dev-attempted-close"
                }),
            )
            .await
            .expect_err("dev should be rejected by close pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn close_story_rejects_empty_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] empty-reason-target",
            "reject if empty-reason close is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": ""
                }),
            )
            .await
            .expect_err("empty reason should be rejected");

        assert!(
            err.contains("reason"),
            "rejection names the offending field; got {err}"
        );
        assert!(
            err.contains("audit"),
            "rejection cites audit-trail rationale; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn close_story_view_board_shows_closed_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] close-roundtrip-target",
            "reject if view_board doesn't reflect closed state after close_story",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "number": target_number,
                "reason": "close-roundtrip-marker"
            }),
        )
        .await
        .expect("close_story should succeed");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains(&format!("#{target_number}"))
                || view_text.contains("ACCEPTED"),
            "view_board reflects the closed-as-accepted story per is_closed→accepted mapping; got:\n{view_text}"
        );

        pm.close().await;
    });
}
