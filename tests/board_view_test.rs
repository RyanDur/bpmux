//! End-to-end test for #193 board view.
//!
//! Tests the board lifecycle: open via /board, verify column layout,
//! navigate with arrow keys, drill into detail, Esc back.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// GIVEN the TUI is running
/// WHEN /board is typed
/// THEN the board view opens showing state column headers
/// AND Esc returns to chat
#[test]
fn board_opens_with_columns_and_esc_returns_to_chat() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    // gh call may take a few seconds on first run
    let has_board = bpmux.wait_for_tui("READY", Duration::from_secs(10))
        || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2))
        || bpmux.wait_for_tui("board", Duration::from_secs(2));
    let screen = bpmux.tui();

    // Board must show at least one state column header
    let has_columns = has_board
        || screen.contains("READY")
        || screen.contains("IN PROGRESS")
        || screen.contains("DELIVERED")
        || screen.contains("ACCEPTED")
        || screen.contains("ICEBOX");
    assert!(
        has_columns,
        "board should show state column headers. screen:\n{screen}"
    );

    // Board should show "board" in the title or status bar
    assert!(
        screen.contains("board"),
        "board title should be visible. screen:\n{screen}"
    );

    // Esc should return to chat — user sees the input prompt
    bpmux.type_key("Esc");
    std::thread::sleep(Duration::from_millis(300));
    let after = bpmux.tui();
    assert!(
        after.contains(">"),
        "Esc should return to chat with input prompt. screen:\n{after}"
    );
    // Board columns should no longer be visible
    let board_gone = !after.contains("READY (")
        && !after.contains("IN PROGRESS (")
        && !after.contains("DELIVERED (");
    assert!(
        board_gone,
        "board should be dismissed after Esc. screen:\n{after}"
    );
}

/// GIVEN the board is open with stories
/// WHEN the user presses Down then Enter
/// THEN the detail view opens showing story metadata
/// AND Esc returns to the board (not chat)
#[test]
fn board_detail_shows_story_and_esc_returns_to_board() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(10))
            || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2)),
        "board should open"
    );

    // Navigate to first story and open detail
    bpmux.type_key("Down");
    std::thread::sleep(Duration::from_millis(100));
    bpmux.type_key("Enter");
    std::thread::sleep(Duration::from_millis(300));

    let detail = bpmux.tui();
    // Detail view should show story metadata
    let has_detail = detail.contains("State:") || detail.contains("Filed:") || detail.contains("#");
    assert!(
        has_detail,
        "detail view should show story metadata. screen:\n{detail}"
    );

    // Esc from detail returns to board (not chat)
    bpmux.type_key("Esc");
    std::thread::sleep(Duration::from_millis(300));
    let back_to_board = bpmux.tui();
    let has_columns = back_to_board.contains("READY")
        || back_to_board.contains("DELIVERED")
        || back_to_board.contains("ACCEPTED");
    assert!(
        has_columns,
        "Esc from detail should return to board. screen:\n{back_to_board}"
    );
}
