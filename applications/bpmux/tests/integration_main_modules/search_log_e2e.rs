//! Agent-side E2E test for `search_log` MCP tool (#476 Phase 1
//! backfill — read archetype, search-results variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — search_log applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Coverage was previously deferred per #483 — the handler parsed
//! the response shape as `Vec<Value>` but `/api/search` returns
//! `{"messages": [...], "stories": [...], "comments": [...]}`. The
//! mismatch made every non-empty search return an error to the
//! caller (the body cloned into Err); empty boards papered over
//! the bug because the parse-failure didn't fire. Fixed in the
//! same commit that adds this test (debt-at-sight pairing).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn search_log_returns_no_results_phrase_for_empty_session() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "search_log",
                serde_json::json!({"query": "nonexistent_marker_xyz"}),
            )
            .await
            .expect("search_log should succeed against an empty session");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("No results"),
            "empty search yields the no-results phrase; got `{text}`"
        );

        pm.close().await;
    });
}

#[test]
fn search_log_returns_matches_with_search_header_when_query_hits() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "SEARCH_LOG_E2E_MARKER_BPF7";
        pm.send(&format!("filing a note with {marker} embedded"))
            .await;

        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = pm
            .call_tool("search_log", serde_json::json!({"query": marker}))
            .await
            .expect("search_log should succeed against a populated session");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("Search:") && text.contains(marker),
            "result names the query in the header AND surfaces the matched marker in the body; got:\n{text}"
        );
        assert!(
            text.contains("End of results"),
            "result includes the end-of-results sentinel; got:\n{text}"
        );

        pm.close().await;
    });
}

#[test]
fn search_log_rejects_missing_query() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("search_log", serde_json::json!({}))
            .await
            .expect_err("missing query should be rejected");

        assert!(
            err.contains("query"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
