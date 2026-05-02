//! Agent-side E2E test for `capture_screen` MCP tool (#476 Phase 1
//! backfill — read archetype, TUI-render-snapshot variant). Uses
//! the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — capture_screen applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Fetches the current TUI screen-buffer from bpmux server's
//! `/api/screen` endpoint.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn capture_screen_returns_text_payload() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("capture_screen", serde_json::json!({}))
            .await
            .expect("capture_screen should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !text.is_empty(),
            "capture_screen returns non-empty text payload (TUI snapshot); got `{text}`"
        );

        dev.close().await;
    });
}
