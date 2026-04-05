//! Bridge Identity Test: Two bridges with different names show distinct identities
//!
//! Proves: when two bridge clients connect with --name dev and --name pm,
//! messages from each appear with their respective identity in the TUI.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn two_bridges_show_distinct_identities() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let pm = TestBridge::connect(&bpmux, "pm").await;

        dev.send("hello from dev").await;
        pm.send("hello from pm").await;

        dev.close().await;
        pm.close().await;
    });

    assert!(
        bpmux.wait_for_tui("hello from pm", Duration::from_secs(5)),
        "pm's message should reach the TUI"
    );
    let tui = bpmux.tui();
    assert!(
        tui.contains("dev:") && tui.contains("hello from dev"),
        "Dev bridge should show as 'dev' in TUI. Screen:\n{tui}"
    );
    assert!(
        tui.contains("pm:") && tui.contains("hello from pm"),
        "PM bridge should show as 'pm' in TUI. Screen:\n{tui}"
    );
}
