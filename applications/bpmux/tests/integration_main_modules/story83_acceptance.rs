//! Story 83 Acceptance Test: a user can view the project roadmap
//! from the TUI.
//!
//! AC1 (roadmap text surfaces in the TUI) is covered end-to-end in
//! tests/view_roadmap_test.rs since the typed `/roadmap` slash was
//! removed in #233 — agents now trigger it via the `view_roadmap`
//! MCP tool. This file keeps AC3 (the long-message truncation +
//! expansion behavior) but drives the surface through the same
//! MCP path so tests don't depend on a removed slash command.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC3: GIVEN view_roadmap surfaces a long document
/// WHEN the content exceeds the viewport
/// THEN truncation indicator shows and Enter expands it
#[test]
fn ac3_roadmap_is_expandable() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        pm.call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed");
        pm.close().await;
    });

    assert!(
        bpmux.wait_for_tui("lines]", Duration::from_secs(3)),
        "Long roadmap should show truncation indicator. Screen:\n{}",
        bpmux.tui()
    );

    // Press Enter on empty input to expand the last (truncated) message
    bpmux.type_line("");
    bpmux
        .wait_for_tui_absent("lines]", Duration::from_secs(5))
        .expect("expanded roadmap should not show truncation indicator");
}
