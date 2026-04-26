//! Agent-side E2E test for `view_comments` MCP tool (#476 Phase 1
//! backfill — read archetype, comment-list variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_comments applicability =
//! {agent: yes, human: yes (drill-in surface in Electron),
//! external: no}. This test covers ONLY agent-side; human-side
//! follows per applicable-class backfill.
//!
//! **Deferred falsifier — multi-comment chronological ordering**:
//! `handle_comment_on_story` calls `refresh_story` BEFORE `mutate_sync`.
//! `refresh_story` calls `store.seed_all` which REPLACES the story in
//! BoardStore with the adapter's version. Adapter
//! (InMemorySourceAdapter via `--fake-tracker`) doesn't store
//! comments, so each refresh wipes prior `free_comments`. Multi-comment
//! tests against `--fake-tracker` only see the most-recent comment.
//! In production with real GitHubSourceAdapter, comments persist on GH
//! and `view_comments` fetches them via `fetch_free_comments` — the
//! chronological-ordering falsifier fires correctly there. Substrate
//! options (refresh-preserve-comments / no-refresh-before-comment /
//! MockOctocrab-mode) deferred per
//! `feedback_yagni_with_premise_discipline`.

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
