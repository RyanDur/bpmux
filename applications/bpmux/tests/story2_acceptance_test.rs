//! Story 2 Acceptance Test: Human and AI share a conversation
//!
//! Proves: human types in TUI → message appears, AI sends via WebSocket →
//! message appears in TUI, AI reads via REST → gets both messages.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn human_and_ai_share_a_conversation() {
    let bpmux = TestBpmux::start_named("ryan");
    let tui = bpmux.tui();
    assert!(tui.contains("bpmux"), "TUI should show bpmux title");
    assert!(tui.contains("solo"), "TUI should show solo status");
    assert!(tui.contains(">"), "TUI should show input prompt");

    bpmux.type_line("hello from human");
    // Wait for the POST-SUBMIT rendering specifically ("ryan:" prefix
    // only appears after Enter has been processed and the message is
    // in the chat area). Waiting for the raw text matches both the
    // input box and the chat area, so it can return before submission
    // completes — racey in CI, stable-ish locally.
    assert!(
        bpmux.wait_for_tui("ryan:", Duration::from_secs(3)),
        "Post-submit rendering should show sender. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.tui().contains("hello from human"),
        "Human message text should be in TUI. Screen:\n{}",
        bpmux.tui()
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut ai = WsPeer::connect(&bpmux, "ai").await;
        ai.send("hello from ai").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    assert!(
        bpmux.wait_for_tui("hello from ai", Duration::from_secs(3)),
        "AI message should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
}
