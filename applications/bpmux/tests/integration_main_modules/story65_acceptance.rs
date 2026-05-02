//! Story 65 Acceptance Test: The owner can see whether each participant is
//! available, busy, or needs help — shown as a colored dot next to their name.
//!
//! AC1: New participant shows a green dot (●) next to their name
//! AC2: Participant setting status to "busy" via activity message changes dot
//! AC3: Participant setting status to "help" via activity message changes dot
//! AC4: DM arrival shows ✉ badge with sender name

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a participant connects and sends a message
/// WHEN the owner looks at the status bar
/// THEN a dot (●) and the participant's short name appear
#[test]
fn ac1_connected_participant_shows_dot_and_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("hello").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("\u{25CF} dev", Duration::from_secs(5)),
        "Status bar should show dot and name for connected peer. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a participant sends an activity message with status "busy"
/// WHEN the status bar renders
/// THEN a dot and their name appear (color is yellow, verified via unit test)
#[test]
fn ac2_busy_status_via_activity_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_activity("busy").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("\u{25CF} dev", Duration::from_secs(5)),
        "Status bar should show dot for busy peer. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN a participant sends an activity message with status "help"
/// WHEN the status bar renders
/// THEN a dot and their name appear (color is red, verified via unit test)
#[test]
fn ac3_help_status_via_activity_message() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_activity("help").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("\u{25CF} dev", Duration::from_secs(5)),
        "Status bar should show dot for help peer. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a DM arrives for the owner
/// WHEN the status bar renders
/// THEN a ✉ badge with the sender's name appears
#[test]
fn ac4_dm_shows_envelope_badge() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send_to("hey owner", &["human"]).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("\u{2709} DM from dev", Duration::from_secs(5)),
        "Status bar should show DM badge. Screen:\n{}",
        bpmux.tui()
    );
}
