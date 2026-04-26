//! Agent-side E2E test for `dev_in_flight` MCP tool (#476 Phase 1
//! backfill — read archetype, boolean-aggregation variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — dev_in_flight applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn dev_in_flight_returns_false_for_empty_board() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("dev_in_flight", serde_json::json!({}))
            .await
            .expect("dev_in_flight should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("\"dev_in_flight\":false"),
            "empty-board returns dev_in_flight=false; got `{text}`"
        );

        dev.close().await;
    });
}

#[test]
fn dev_in_flight_returns_well_formed_boolean_payload() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("dev_in_flight", serde_json::json!({}))
            .await
            .expect("dev_in_flight should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        let parsed: serde_json::Value =
            serde_json::from_str(&text).expect("dev_in_flight returns JSON");
        assert!(
            parsed["dev_in_flight"].is_boolean(),
            "payload exposes dev_in_flight as boolean; got {parsed}"
        );
        assert!(
            !text.contains("\"number\"") && !text.contains("\"title\""),
            "payload omits per-story identifiers (privacy-by-design); got `{text}`"
        );

        dev.close().await;
    });
}
