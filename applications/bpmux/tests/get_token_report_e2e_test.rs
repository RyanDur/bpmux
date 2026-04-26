//! Agent-side E2E test for `get_token_report` MCP tool (#476 Phase 1
//! backfill — read archetype, file-backed report-aggregation).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — get_token_report applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_token_report_returns_well_formed_json() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_token_report", serde_json::json!({}))
            .await
            .expect("get_token_report should succeed");

        let parsed = match &result {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("get_token_report returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_object() || parsed.is_array(),
            "payload is structured JSON; got {parsed:?}"
        );

        dev.close().await;
    });
}
