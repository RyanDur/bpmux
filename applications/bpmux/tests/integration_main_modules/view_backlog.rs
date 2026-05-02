//! #286 acceptance test: an agent calls `view_backlog`; the project
//! panel opens in Ryan's TUI with the board section visible. Agent-
//! triggered display; Ryan doesn't type anything.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_backlog_returns_json_summary_and_opens_panel() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        // `call_tool` auto-parses JSON responses, so the returned
        // value is the shape the agent sees. Assert the full contract
        // so a future shape change trips the test.
        let parsed = pm
            .call_tool("view_backlog", serde_json::json!({}))
            .await
            .expect("view_backlog should succeed");
        assert!(
            parsed["ready_count"].is_number(),
            "payload must expose ready_count. got: {parsed}"
        );
        assert!(
            parsed["delivered_count"].is_number(),
            "payload must expose delivered_count. got: {parsed}"
        );
        assert!(
            parsed["icebox_count"].is_number(),
            "payload must expose icebox_count. got: {parsed}"
        );
        assert!(
            parsed["ready_samples"].is_array(),
            "payload must expose ready_samples array. got: {parsed}"
        );
        assert!(
            parsed["delivered_samples"].is_array(),
            "payload must expose delivered_samples array. got: {parsed}"
        );

        // Ryan's TUI should show the project panel. Header is " bpmux "
        // in ratatui's border rendering — that's what project panel
        // draws on top of the board section.
        assert!(
            bpmux.wait_for_tui("bpmux", Duration::from_secs(3)),
            "project panel header should appear in TUI. screen:\n{}",
            bpmux.tui()
        );

        pm.close().await;
    });
}
