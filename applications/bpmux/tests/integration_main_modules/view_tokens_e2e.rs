//! Agent-side E2E test for `view_tokens` MCP tool (#476 Phase 1
//! backfill — read archetype, marker-only variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_tokens applicability =
//! {agent: yes (returns confirmation string + fires marker), human:
//! yes (TUI overlay opens), external: no}. This test covers ONLY
//! agent-side; the TUI-side marker behavior is verified by other
//! capture_screen-based human-side tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_tokens_returns_open_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_tokens", serde_json::json!({}))
            .await
            .expect("view_tokens should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("token") && text.contains("opened"),
            "view_tokens returns the confirmation string per the marker-only contract; got `{text}`"
        );

        pm.close().await;
    });
}
