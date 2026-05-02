//! Agent-side E2E test for `view_board` MCP tool (#476 Phase 1
//! backfill — read archetype, board-render variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — view_board applicability =
//! {agent: yes, human: no (board has dedicated TUI surface), external: no}.
//! Single-class agent-only.
//!
//! view_board's dispatch path is exercised indirectly by the 6
//! multi-role roundtrip closures (start/deliver/fail/pass/accept/
//! reject), but those tests assert post-mutation state. This test
//! covers the dispatch + section-rendering contract directly:
//! all expected section headers always present, regardless of board
//! emptiness.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_board_renders_all_section_headers_on_empty_board() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");

        let text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        // Canonical view_board projection per #517's owner-semantic
        // model: READY-FOR-ACCEPT / NEEDS-FIXES / REJECTED stories
        // bucket into READY rather than rendering as separate columns.
        // The 6-column header set below matches what `column_for`
        // emits today.
        for header in [
            "ICEBOX",
            "READY",
            "IN PROGRESS",
            "DESK CHECK",
            "DELIVERED",
            "ACCEPTED",
        ] {
            assert!(
                text.contains(header),
                "view_board renders section header `{header}`; got:\n{text}"
            );
        }

        pm.close().await;
    });
}
