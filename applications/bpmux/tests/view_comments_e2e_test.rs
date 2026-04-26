//! Agent-side E2E test for `view_comments` MCP tool (#476 Phase 1
//! backfill — read archetype, comment-list variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_comments applicability =
//! {agent: yes, human: yes (drill-in surface in Electron),
//! external: no}. This test covers ONLY agent-side; human-side
//! follows per applicable-class backfill.
//!
//! Multi-comment chronological-ordering falsifier remains deferred:
//! #485 closed the comment-persistence-asymmetry (adapter now stores
//! comments + populates them in fetch_story_metadata). But there's a
//! second substrate-race: `push_event` (which writes to adapter)
//! fires async via `drain_pending` background task; rapid back-to-back
//! comment_on_story calls may refresh from the adapter BEFORE drain
//! has propagated the prior comment. Closing the multi-comment-
//! ordering falsifier requires either synchronous-push-on-mutate OR
//! mutate-before-refresh in handle_comment_on_story OR a test-only
//! force-drain primitive. Tracked for follow-up substrate-Chore.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn view_comments_returns_empty_array_for_uncommented_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] view-comments-empty-target",
            "reject if view_comments returns spurious entries on uncommented story",
        )
        .await;

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
            view_text.trim() == "[]" || view_text.contains("\"comments\":[]"),
            "uncommented story returns empty comments array; got `{view_text}`"
        );

        pm.close().await;
    });
}

#[test]
fn view_comments_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("view_comments", serde_json::json!({}))
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
fn view_comments_returns_error_for_non_existent_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("view_comments", serde_json::json!({"number": 999999}))
            .await
            .expect_err("non-existent story should be rejected");

        assert!(
            err.contains("not found") || err.contains("999999"),
            "rejection names the missing story; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn view_comments_reflects_single_comment_with_author_and_timestamp() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] view-comments-shape-target",
            "reject if comment payload omits author or timestamp",
        )
        .await;

        let marker = "VIEW_COMMENTS_SHAPE_MARKER_S1";
        pm.call_tool(
            "comment_on_story",
            serde_json::json!({"number": target_number, "body": marker}),
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
            "view_comments includes the comment body verbatim; got:\n{view_text}"
        );
        assert!(
            view_text.contains("\"author\""),
            "comment payload exposes author field; got:\n{view_text}"
        );
        assert!(
            view_text.contains("\"created_at\""),
            "comment payload exposes created_at timestamp; got:\n{view_text}"
        );

        pm.close().await;
    });
}
