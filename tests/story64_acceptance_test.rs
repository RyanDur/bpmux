//! Story 64 Acceptance Test: Command discovery and visual feedback
//!
//! AC1: Typing `/` shows a command popup
//! AC2: Continuing to type filters the popup
//! AC3: Command keyword is colored in the input line
//! AC4: No matches hides the popup
//! AC5: Regular messages show no popup

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a user types `/`
/// THEN a popup showing available commands appears
#[test]
fn ac1_slash_shows_command_popup() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("bpmux", Duration::from_secs(5)),
        "TUI should be ready. Screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("/");
    assert!(
        bpmux.wait_for_tui("/who", Duration::from_secs(3)),
        "Popup should show /who command. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("/view", Duration::from_secs(3)),
        "Popup should show /view command. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a user types `/to`
/// THEN the popup filters to matching commands only
#[test]
fn ac2_typing_filters_popup() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("bpmux", Duration::from_secs(5)),
        "TUI should be ready. Screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("/to");
    assert!(
        bpmux.wait_for_tui("/to", Duration::from_secs(3)),
        "Popup should show /to. Screen:\n{}",
        bpmux.tui()
    );
    // /who should NOT match (doesn't start with /to)
    assert!(
        !bpmux.wait_for_tui("list participants", Duration::from_secs(1)),
        "Popup should not show /who when typing /to. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN no commands match
/// THEN the popup disappears
#[test]
fn ac4_no_matches_hides_popup() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("bpmux", Duration::from_secs(5)),
        "TUI should be ready. Screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("/xyz");
    assert!(
        !bpmux.wait_for_tui("show this help", Duration::from_secs(1)),
        "No command descriptions should appear for /xyz. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN a user types a regular message
/// THEN no popup appears
#[test]
fn ac5_regular_message_no_popup() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("bpmux", Duration::from_secs(5)),
        "TUI should be ready. Screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("hello world");
    assert!(
        !bpmux.wait_for_tui("show this help", Duration::from_secs(1)),
        "Command descriptions should not appear for regular text. Screen:\n{}",
        bpmux.tui()
    );
}
