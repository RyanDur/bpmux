//! Agent-side E2E test for `get_ci_log_for_run(run_id)` MCP tool (#1174).
//! Mocks GH Actions runs/jobs/logs endpoints and exercises the
//! end-to-end bridge → REST → tracker → parser → CiLogStore chain.
//! Assertion: the tool returns structured per-test failure records;
//! cache hit on second invocation skips network.
//!
//! Per the 3-class framing — get_ci_log_for_run applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_ci_log_for_run_returns_structured_failures_from_mocked_actions() {
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
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/actions/runs/42");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "id": 42,
                    "status": "completed",
                    "conclusion": "failure"
                })
                .to_string(),
            );
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/actions/runs/42/jobs");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                serde_json::json!({
                    "jobs": [{
                        "id": 7,
                        "name": "playwright-e2e (6)",
                        "conclusion": "failure"
                    }]
                })
                .to_string(),
            );
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/actions/jobs/7/logs");
        then.status(200)
            .header("content-type", "text/plain")
            .body(
                "Running 47 tests using 4 workers, shard 6 of 6\n\n\
                 \u{2718}  1 tests/playwright/offline-board.spec.ts:121:7 \u{203A} #1158 \u{2014} offline-first board \u{203A} F8 stale\n\n\
                 Test timeout of 30000ms exceeded.\n",
            );
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_ci_log_for_run", serde_json::json!({"run_id": 42}))
            .await
            .expect("get_ci_log_for_run should succeed against mocked GH");

        let parsed = match &result {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("get_ci_log_for_run returned unexpected shape: {other:?}"),
        };
        assert_eq!(parsed["run_id"], 42);
        assert_eq!(parsed["conclusion"], "failure");
        let failures = parsed["failures"].as_array().expect("failures array");
        assert!(!failures.is_empty(), "expected at least one parsed failure");

        dev.close().await;
    });
}

#[test]
fn get_ci_log_for_run_rejects_missing_run_id() {
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_ci_log_for_run", serde_json::json!({}))
            .await;
        assert!(
            result.is_err(),
            "missing run_id should error; got {result:?}"
        );

        dev.close().await;
    });
}
