//! Agent-side E2E test for `comment_on_story` MCP tool (#476 Phase 1
//! backfill — mutation archetype, append-only variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — comment_on_story applicability =
//! {agent: yes, human: yes (chat surface in Electron), external: yes
//! (gh issue comment)}. This test covers ONLY the agent-side class;
//! human-side + external-mutation E2E follow per applicable-class
//! backfill discipline.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn comment_on_story_returns_message_naming_target() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] comment-target",
            "reject if comment-on-non-existent-story silently succeeds",
        )
        .await;

        let result = pm
            .call_tool(
                "comment_on_story",
                serde_json::json!({
                    "number": target_number,
                    "body": "first comment marker for E2E"
                }),
            )
            .await
            .expect("comment_on_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("commented"),
            "happy-path message names the affordance; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn comment_on_story_rejects_missing_body() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] body-validation-target",
            "reject if missing-body comment silently succeeds",
        )
        .await;

        let err = pm
            .call_tool(
                "comment_on_story",
                serde_json::json!({
                    "number": target_number
                }),
            )
            .await
            .expect_err("missing body should be rejected");

        assert!(
            err.contains("body"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn comment_on_story_rejects_empty_body() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] empty-body-target",
            "reject if empty-body comment silently succeeds",
        )
        .await;

        let err = pm
            .call_tool(
                "comment_on_story",
                serde_json::json!({
                    "number": target_number,
                    "body": ""
                }),
            )
            .await
            .expect_err("empty body should be rejected");

        assert!(
            err.contains("body"),
            "rejection names the offending field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn comment_on_story_view_comments_reflects_new_comment() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] roundtrip-comment-target",
            "reject if view_comments doesn't reflect appended comment",
        )
        .await;

        let marker = "ROUNDTRIP_COMMENT_MARKER_E1";
        pm.call_tool(
            "comment_on_story",
            serde_json::json!({
                "number": target_number,
                "body": marker
            }),
        )
        .await
        .expect("comment_on_story should succeed");

        let view = pm
            .call_tool(
                "view_comments",
                serde_json::json!({"number": target_number}),
            )
            .await
            .expect("view_comments should succeed");
        let view_text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains(marker),
            "view_comments reflects the new comment via real MCP roundtrip; got:\n{view_text}"
        );

        pm.close().await;
    });
}
