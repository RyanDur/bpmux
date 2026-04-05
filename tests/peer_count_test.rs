//! Peer Count Test: TUI status bar shows connected peer count
//!
//! Proves: when a WebSocket client connects and identifies,
//! the TUI status bar changes from "solo" to "1 connected".

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn status_bar_shows_peer_count() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.tui().contains("solo"),
        "TUI should start in solo mode"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let _peer = WsPeer::connect(&bpmux, "test-peer").await;
        // Keep the WebSocket alive long enough for the TUI to update.
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    assert!(
        bpmux.wait_for_tui("connected", Duration::from_secs(3)),
        "Status bar should reflect a connected peer. Screen:\n{}",
        bpmux.tui()
    );
}
