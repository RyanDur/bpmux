//! Story 59 Acceptance Test: The owner can see whether each participant is busy, idle, or blocked
//!
//! AC1: Status bar shows each participant's current state
//! AC2: New participant shows as "connected"
//! AC3: `/status blocked [reason]` sets blocked state with reason
//! AC4: `/status working on X` sets self-declared status
//! AC5: Blocked participant declaring a new status clears blocked

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1 + AC2: GIVEN a participant sends a message
/// WHEN the owner looks at the TUI status bar
/// THEN that participant shows as "connected"
#[test]
fn ac2_new_participant_shows_connected() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("hello").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [connected]", Duration::from_secs(5)),
        "Status bar should show dev as connected. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN a participant sends `/status blocked [reason]`
/// WHEN the status bar renders
/// THEN that participant shows as "blocked" with the reason visible
#[test]
fn ac3_status_blocked_shows_in_status_bar() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("/status blocked waiting on design review").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [blocked", Duration::from_secs(5)),
        "Status bar should show dev as blocked. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a participant sends `/status working on #62`
/// WHEN the status bar renders
/// THEN that participant shows with the self-declared status
#[test]
fn ac4_self_declared_status_shows_in_status_bar() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("/status working on #62").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [working on #62]", Duration::from_secs(5)),
        "Status bar should show dev's self-declared status. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN a blocked participant declares a new status
/// WHEN the status bar renders
/// THEN the blocked state is replaced
#[test]
fn ac5_new_status_replaces_blocked() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("/status blocked waiting on design").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [blocked", Duration::from_secs(5)),
        "Should show blocked first. Screen:\n{}",
        bpmux.tui()
    );

    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("/status working on #62").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [working on #62]", Duration::from_secs(5)),
        "Status bar should show new status after unblocking. Screen:\n{}",
        bpmux.tui()
    );
}
