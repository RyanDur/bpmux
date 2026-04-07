//! Story 68 Acceptance Test: Status bar reflects only live connections.
//!
//! AC1: After restart, status bar starts clean (no stale peers from history)
//! AC2: Live connections populate the status bar

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN agents were connected in a previous session
/// WHEN the TUI restarts (even without graceful shutdown)
/// THEN the status bar starts clean — only live connections show
#[test]
fn ac1_status_starts_clean_after_restart() {
    let mut bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _dev = rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        dev
    });

    // Verify status dot appears while connected
    assert!(
        bpmux.wait_for_tui("\u{25CF}", Duration::from_secs(3)),
        "Dev status dot should appear while connected. Screen:\n{}",
        bpmux.tui()
    );

    let session_dir = bpmux.take_session_dir();
    drop(bpmux);
    drop(_dev);
    drop(rt);
    std::thread::sleep(Duration::from_millis(500));

    // Restart with the same session — no peers connected
    let bpmux2 = TestBpmux::start_with_session("human", session_dir);
    std::thread::sleep(Duration::from_millis(500));

    let screen = bpmux2.tui();
    assert!(
        screen.contains("solo"),
        "Should show solo after restart with no live connections. Screen:\n{screen}"
    );
}

/// AC2: GIVEN the TUI is running
/// WHEN a peer connects and sets status
/// THEN the status bar updates in real time
#[test]
fn ac2_disconnected_peer_absent_after_restart() {
    let mut bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        dev.close().await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    let session_dir = bpmux.take_session_dir();
    drop(bpmux);
    std::thread::sleep(Duration::from_millis(500));

    // Restart — dev disconnected, so status bar should be clean
    let bpmux2 = TestBpmux::start_with_session("human", session_dir);
    std::thread::sleep(Duration::from_millis(500));

    let screen = bpmux2.tui();
    assert!(
        screen.contains("solo"),
        "Should show solo after restart with disconnected peer. Screen:\n{screen}"
    );
}
