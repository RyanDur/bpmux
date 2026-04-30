//! Agent-side E2E test for `list_ready` MCP tool (#476 Phase 1
//! backfill — read archetype, state-filtered list variant). Uses
//! the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — list_ready applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Returns array of stories in Ready state with priority ordering
//! applied. Empty array for empty/no-Ready board.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn list_ready_returns_empty_array_for_empty_board() {
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
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("list_ready", serde_json::json!({}))
            .await
            .expect("list_ready should succeed");

        let parsed = match &result {
            serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("list_ready returned unexpected shape: {other:?}"),
        };
        let arr = parsed.as_array().expect("array");
        assert!(
            arr.is_empty(),
            "empty-board returns empty Ready-list; got {arr:?}"
        );

        dev.close().await;
    });
}

#[test]
fn list_ready_returns_well_formed_array_payload() {
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
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("list_ready", serde_json::json!({}))
            .await
            .expect("list_ready should succeed");

        let parsed = match &result {
            serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("list_ready returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_array(),
            "list_ready returns a JSON array; got {parsed:?}"
        );

        dev.close().await;
    });
}
