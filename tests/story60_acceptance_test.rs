//! Story 60 Acceptance Test: A first-time user understands the basics within
//! 60 seconds of starting bpmux.
//!
//! AC3: Help keybinding shows overlay
//! AC4: Any key dismisses help overlay

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// AC3: GIVEN a user is in the TUI
/// WHEN they press `?` with empty input
/// THEN a help overlay appears
#[test]
fn ac3_help_keybinding_shows_overlay() {
    let bpmux = TestBpmux::start();
    // Press ? to show help overlay
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "Should show help overlay. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN the help overlay is visible
/// WHEN the user presses any key
/// THEN the overlay closes
#[test]
fn ac4_any_key_dismisses_help() {
    let bpmux = TestBpmux::start();
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui("Press any key to close", Duration::from_secs(5)),
        "Help should be visible. Screen:\n{}",
        bpmux.tui()
    );
    // Press a key to dismiss
    bpmux.type_chars("x");
    // Wait briefly, then verify overlay is gone
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        !bpmux.tui().contains("Press any key to close"),
        "Help overlay should be dismissed. Screen:\n{}",
        bpmux.tui()
    );
}
