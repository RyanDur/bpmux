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

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_backlog_returns_zero_counts_for_empty_board() {
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
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
    use harness::mock_gh::issue_json;
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                Value::Array(vec![
                    issue_json(801, "Pre-seeded ready", &["ready"], None),
                    issue_json(802, "Pre-seeded in-progress", &["in progress"], None),
                    issue_json(803, "Pre-seeded delivered", &["delivered"], None),
                ])
                .to_string(),
            );
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

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
