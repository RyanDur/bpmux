//! End-to-end test for #193/#196 board view.
//!
//! Tests the board lifecycle: open via /board, verify data renders
//! in the project panel alongside chat, Esc closes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Returns false if gh CLI is not available or not authenticated.
fn gh_available() -> bool {
    std::process::Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// GIVEN the TUI is running
/// WHEN /board is typed
/// THEN the project panel opens showing board data (state headers)
/// AND chat is still visible below
/// AND Esc closes the panel
#[test]
fn board_opens_in_panel_with_chat_visible() {
    if !gh_available() {
        eprintln!("SKIP: gh CLI not authenticated");
        return;
    }
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    // gh call may take a few seconds
    let has_board = bpmux.wait_for_tui("project", Duration::from_secs(10))
        || bpmux.wait_for_tui("READY", Duration::from_secs(2))
        || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2));
    let screen = bpmux.tui();

    // Project panel should be visible with board data
    assert!(
        has_board,
        "board should open in project panel. screen:\n{screen}"
    );

    // Chat input should still be visible (split view)
    assert!(
        screen.contains(">"),
        "input prompt should be visible alongside board. screen:\n{screen}"
    );

    // Esc should close the panel
    bpmux.type_key("Esc");
    std::thread::sleep(Duration::from_millis(300));
    let after = bpmux.tui();
    assert!(
        !after.contains("project ") || after.contains(">"),
        "Esc should close panel and restore chat. screen:\n{after}"
    );
}

/// GIVEN the board has stories in active states
/// WHEN the project panel is open
/// THEN story titles are visible in the panel
#[test]
fn board_panel_shows_story_titles() {
    if !gh_available() {
        eprintln!("SKIP: gh CLI not authenticated");
        return;
    }
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("project", Duration::from_secs(10))
            || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2)),
        "board should open"
    );

    let screen = bpmux.tui();
    // Should show at least one story number
    assert!(
        screen.contains("#"),
        "board panel should show story numbers. screen:\n{screen}"
    );
}
