//! Story 3 Acceptance Test: WebSocket client participates in conversation
//!
//! Proves: WebSocket client connects → receives human's message →
//! sends a response → response appears in TUI. Uses real collaborators,
//! no mocks, no external APIs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn ws_client_participates_in_conversation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut bot = WsPeer::connect(&bpmux, "bot").await;

        bpmux.type_line("hello from human");

        let (_from, text) = bot.recv_message(Duration::from_secs(5)).await;
        assert_eq!(text, "hello from human");

        bot.send("hello from bot").await;
    });

    assert!(
        bpmux.wait_for_tui("hello from bot", Duration::from_secs(5)),
        "Bot response should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
}
