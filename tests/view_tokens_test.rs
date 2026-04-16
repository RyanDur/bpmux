//! #225 acceptance test: an agent calls `view_tokens`; the tokens
//! overlay opens in Ryan's TUI. Agent-triggered display; Ryan
//! doesn't type anything.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_tokens_surfaces_overlay_in_tui() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = pm
            .call_tool("view_tokens", serde_json::json!({}))
            .await
            .expect("view_tokens should succeed");
        let msg = result.as_str().unwrap_or("");
        assert!(
            msg.contains("token view"),
            "tool should confirm token view opened. got: {msg}"
        );

        // Ryan's TUI should show the overlay. The overlay title is
        // " tokens " (rendered with spaces and border).
        assert!(
            bpmux.wait_for_tui("tokens", Duration::from_secs(3)),
            "overlay should appear in TUI. screen:\n{}",
            bpmux.tui()
        );

        pm.close().await;
    });
}
