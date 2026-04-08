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
        "Status bar should show peer dot. Screen:\n{}",
        bpmux.tui()
    );
}
