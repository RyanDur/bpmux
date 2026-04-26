//! Agent-side E2E test for `view_velocity` MCP tool (#476 Phase 1
//! backfill — read archetype, log-derived metrics variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_velocity applicability =
//! {agent: yes (returns metrics JSON), human: yes (TUI panel opens),
//! external: no}. This test covers ONLY agent-side; the TUI marker
//! is verified by capture_screen-based human-side tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_velocity_returns_metrics_json_for_empty_session() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_velocity", serde_json::json!({}))
            .await
            .expect("view_velocity should succeed");

        assert!(
            result.is_object(),
            "view_velocity returns a JSON object; got {result:?}"
        );

        pm.close().await;
    });
}
