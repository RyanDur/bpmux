//! Story 68 Acceptance Test: Status dots survive TUI restart.
//!
//! AC1: After restart, status bar shows status of connected participants
//! AC2: Disconnected participants don't show after restart

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN agents have set status and the TUI is killed (crash scenario)
/// WHEN the TUI restarts
/// THEN the status bar shows the previously known status
#[test]
fn ac1_status_survives_restart() {
    // First session: agent connects and sets status
    let mut bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _dev = rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        dev // keep peer alive — if it drops, bpmux persists a "left" message
    });

    // Verify status dot appears
    assert!(
        bpmux.wait_for_tui("\u{25CF}", Duration::from_secs(3)),
        "Dev status dot should appear in status bar. Screen:\n{}",
        bpmux.tui()
    );

    // Take session dir while peer is still connected, then kill bpmux (simulates crash).
    // The peer "left" message won't be persisted because bpmux is killed with SIGKILL.
    let session_dir = bpmux.take_session_dir();
    drop(bpmux); // SIGKILL — no graceful shutdown
    drop(_dev); // now safe to drop the peer
    drop(rt);
    std::thread::sleep(Duration::from_millis(500));

    // Restart with the same session
    let bpmux2 = TestBpmux::start_with_session("human", session_dir);

    // The status dot for dev should still be visible (no "left" in the log)
    assert!(
        bpmux2.wait_for_tui("dev", Duration::from_secs(3)),
        "Dev status should survive restart. Screen:\n{}",
        bpmux2.tui()
    );
}

/// AC2: GIVEN a participant disconnected before restart
/// WHEN the TUI restarts
/// THEN that participant does not appear in the status bar
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

    // Take session dir and stop first instance
    let session_dir = bpmux.take_session_dir();
    drop(bpmux);
    std::thread::sleep(Duration::from_millis(500));

    // Restart with the same session
    let bpmux2 = TestBpmux::start_with_session("human", session_dir);
    std::thread::sleep(Duration::from_millis(500));

    // Dev should NOT appear — they disconnected before restart
    let screen = bpmux2.tui();
    // The status bar should show "solo" (no peers)
    assert!(
        screen.contains("solo"),
        "Should show solo after restart with disconnected peer. Screen:\n{screen}"
    );
}
