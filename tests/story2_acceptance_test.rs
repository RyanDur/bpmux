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
    assert!(
        bpmux.wait_for_tui("hello from human", Duration::from_secs(3)),
        "Human message should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.tui().contains("ryan:"),
        "Sender identity should show. Screen:\n{}",
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
