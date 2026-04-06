//! Story 59 Acceptance Test: The owner can see whether each participant is busy, idle, or blocked
//!
//! AC1: Status bar shows each participant's current state
//! AC2: Participant who sent a message within 60s shows as "active"
//! AC3: Participant with no message in >60s shows as "idle"
//! AC4: `/status blocked [reason]` sets blocked state with reason
//! AC5: Blocked participant sending a regular message returns to "busy"

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1 + AC2: GIVEN a participant sends a message
/// WHEN the owner looks at the TUI status bar
/// THEN that participant shows as "active"
#[test]
fn ac2_recent_message_shows_busy() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("working on it").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [active]", Duration::from_secs(5)),
        "Status bar should show dev as busy after sending a message. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a participant sends `/status blocked [reason]`
/// WHEN the status bar renders
/// THEN that participant shows as "blocked" with the reason visible
#[test]
fn ac4_status_blocked_shows_in_status_bar() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // Send a message with the blocked convention
        dev.send("/status blocked waiting on design review").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [blocked", Duration::from_secs(5)),
        "Status bar should show dev as blocked. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN a blocked participant resumes activity
/// WHEN the status bar renders
/// THEN that participant's state returns to "busy"
#[test]
fn ac5_blocked_participant_returns_to_busy_on_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // First, set blocked
        dev.send("/status blocked waiting on design").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    // Confirm blocked shows
    assert!(
        bpmux.wait_for_tui("dev [blocked", Duration::from_secs(5)),
        "Should show blocked first. Screen:\n{}",
        bpmux.tui()
    );

    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // Send a regular message
        dev.send("unblocked now, resuming work").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("dev [active]", Duration::from_secs(5)),
        "Status bar should return to busy after regular message. Screen:\n{}",
        bpmux.tui()
    );
}
