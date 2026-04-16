//! Peer Presence Test: TUI status bar shows peer dots
//!
//! Proves: when a WebSocket client connects and identifies,
//! the TUI status bar shows a dot for that peer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn status_bar_shows_peer_dot() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.tui().contains("solo"),
        "TUI should start in solo mode"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _peer = rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "test-peer").await;
        peer.send_activity("available").await;
        tokio::time::sleep(Duration::from_secs(1)).await;
        peer
    });

    assert!(
        bpmux.wait_for_tui("test-peer", Duration::from_secs(3)),
        "peer name should appear in status bar. Screen:\n{}",
        bpmux.tui()
    );

    // Restrict the check to the status bar region so a match of
    // "test-peer" in the chat body (e.g., from a system "test-peer
    // joined" message) can't satisfy the assertion. The status bar
    // is the last line of the frame per ui.rs layout. The dot glyph
    // is ACTIVE (U+25CF) from icons.rs and renders immediately
    // before the peer name.
    let screen = bpmux.tui();
    let status_bar = screen.lines().last().unwrap_or("");
    assert!(
        status_bar.contains('\u{25CF}'),
        "status bar should render the ACTIVE dot glyph. status bar: `{status_bar}`"
    );
    assert!(
        status_bar.contains("test-peer"),
        "peer name should be adjacent to the dot in the status bar. status bar: `{status_bar}`"
    );
    // Solo mode indicator must be gone — a peer is connected now.
    assert!(
        !status_bar.contains("solo"),
        "status bar should no longer say `solo` after peer connects. \
         status bar: `{status_bar}`"
    );
}
