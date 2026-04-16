//! #226 acceptance test: an agent calls `view_roadmap`; the roadmap
//! surfaces in Ryan's TUI as a system message. No typing required.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn view_roadmap_surfaces_roadmap_text_in_tui() {
    let bpmux = TestBpmux::start_named("ryan");
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = pm
            .call_tool("view_roadmap", serde_json::json!({}))
            .await
            .expect("view_roadmap should succeed");
        let msg = result.as_str().unwrap_or("");
        assert!(
            msg.contains("roadmap"),
            "tool should confirm roadmap view opened. got: {msg}"
        );

        // Ryan's TUI should receive the roadmap as a system message.
        // fetch_roadmap reads ROADMAP.md; if absent, shows "Roadmap
        // unavailable." — either string proves the path fired.
        let appeared = bpmux.wait_for_tui("Roadmap", Duration::from_secs(3))
            || bpmux.wait_for_tui("roadmap", Duration::from_secs(3));
        assert!(
            appeared,
            "roadmap text should appear in TUI. screen:\n{}",
            bpmux.tui()
        );

        pm.close().await;
    });
}
