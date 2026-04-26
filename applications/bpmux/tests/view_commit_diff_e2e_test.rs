//! Agent-side E2E test for `view_commit_diff` MCP tool (#476 Phase 1
//! backfill — read archetype, git-backed). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_commit_diff applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Runs `git show --format=fuller --patch <sha>` against the workspace
//! root. Tests use HEAD as a stable, always-present ref.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_commit_diff_returns_diff_for_head() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("view_commit_diff", serde_json::json!({"sha": "HEAD"}))
            .await
            .expect("view_commit_diff should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("commit ") || text.contains("Author:") || text.contains("Date:"),
            "diff payload includes git-show preamble; got `{text}`"
        );

        dev.close().await;
    });
}

#[test]
fn view_commit_diff_rejects_invalid_sha() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("view_commit_diff", serde_json::json!({"sha": "; rm -rf /"}))
            .await
            .expect_err("path-traversal-shaped sha should be rejected");

        assert!(
            err.contains("invalid sha") || err.contains("invalid ref") || err.contains("invalid"),
            "rejection cites unsafe ref; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn view_commit_diff_rejects_missing_sha() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("view_commit_diff", serde_json::json!({}))
            .await
            .expect_err("missing sha should be rejected");

        assert!(
            err.contains("sha"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn view_commit_diff_truncates_at_max_bytes() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool(
                "view_commit_diff",
                serde_json::json!({"sha": "HEAD", "max_bytes": 100}),
            )
            .await
            .expect("view_commit_diff should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.len() <= 1024,
            "max_bytes=100 produces a small payload (allowing truncation-marker overhead); got len {}",
            text.len()
        );

        dev.close().await;
    });
}
