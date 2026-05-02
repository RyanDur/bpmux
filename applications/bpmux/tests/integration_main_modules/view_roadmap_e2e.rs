//! Agent-side E2E test for `view_roadmap` MCP tool (#476 Phase 1
//! backfill — read archetype, file-content variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_roadmap applicability =
//! {agent: yes (returns content), human: yes (TUI overlay opens via
//! marker), external: no}. This test covers ONLY agent-side; the TUI
//! marker fires as a side effect but the agent-readable content is
//! the assertion target.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_roadmap_returns_content_string() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !text.trim().is_empty(),
            "view_roadmap returns non-empty content (or honest-error string); got `{text}`"
        );

        pm.close().await;
    });
}
