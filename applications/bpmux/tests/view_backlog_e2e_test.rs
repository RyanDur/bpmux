//! Agent-side E2E test for `view_backlog` MCP tool (#476 Phase 1
//! backfill — read archetype, summary-counts variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_backlog applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Returns JSON object with state-bucketed counts and ready/delivered
//! samples. Fixture-seeded variant proves bucket-mapping correctness
//! against pre-known board state.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::io::Write;
use std::time::Duration;

#[test]
fn view_backlog_returns_zero_counts_for_empty_board() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_backlog", serde_json::json!({}))
            .await
            .expect("view_backlog should succeed");

        assert_eq!(
            result["ready_count"], 0,
            "empty-board has zero ready stories; got {result:?}"
        );
        assert_eq!(
            result["in_progress_count"], 0,
            "empty-board has zero in-progress stories; got {result:?}"
        );
        assert_eq!(
            result["delivered_count"], 0,
            "empty-board has zero delivered stories; got {result:?}"
        );
        assert_eq!(
            result["accepted_count"], 0,
            "empty-board has zero accepted stories; got {result:?}"
        );
        assert_eq!(
            result["icebox_count"], 0,
            "empty-board has zero icebox stories; got {result:?}"
        );

        pm.close().await;
    });
}

#[test]
fn view_backlog_reflects_seeded_states_via_fixture() {
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[
                {"number":801,"title":"Pre-seeded ready","state":"OPEN","labels":[{"name":"ready"}]},
                {"number":802,"title":"Pre-seeded in-progress","state":"OPEN","labels":[{"name":"in progress"}]},
                {"number":803,"title":"Pre-seeded delivered","state":"OPEN","labels":[{"name":"delivered"}]}
            ]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_board_fixture(&bpmux, "pm", &fixture_path).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_backlog", serde_json::json!({}))
            .await
            .expect("view_backlog should succeed");

        assert_eq!(
            result["ready_count"], 1,
            "fixture-seeded board has one ready story; got {result:?}"
        );
        assert_eq!(
            result["in_progress_count"], 1,
            "fixture-seeded board has one in-progress story; got {result:?}"
        );
        assert_eq!(
            result["delivered_count"], 1,
            "fixture-seeded board has one delivered story; got {result:?}"
        );

        let ready_samples = result["ready_samples"]
            .as_array()
            .expect("ready_samples is an array");
        assert!(
            ready_samples
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s.contains("#801"))),
            "ready_samples names the seeded #801 story; got {ready_samples:?}"
        );

        pm.close().await;
    });
}
