//! End-to-end test for #192 command palette.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// GIVEN bpmux is running
/// WHEN the user presses Ctrl+K
/// THEN the command palette overlay appears with the actions list
#[test]
fn ctrl_k_opens_command_palette() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_ctrl('k');
    assert!(
        bpmux.wait_for_tui("actions", Duration::from_secs(3)),
        "command palette should open. screen:\n{}",
        bpmux.tui()
    );
    // Palette should show available commands
    let screen = bpmux.tui();
    assert!(screen.contains("tokens"), "tokens action missing");
    assert!(screen.contains("focus"), "focus action missing");
}

/// GIVEN the palette is open
/// WHEN the user types "tok"
/// THEN the list filters to show "tokens" and "new topic"
#[test]
fn palette_filters_actions_on_input() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_ctrl('k');
    assert!(
        bpmux.wait_for_tui("actions", Duration::from_secs(3)),
        "palette should open"
    );
    bpmux.type_chars("tok");
    assert!(
        bpmux.wait_for_tui("tokens", Duration::from_secs(5)),
        "tokens should match 'tok'. screen:\n{}",
        bpmux.tui()
    );
}

/// GIVEN the palette is open with a filtered selection
/// WHEN the user presses Enter
/// THEN the selected action runs and the palette closes
#[test]
fn palette_enter_runs_selected_action() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_ctrl('k');
    assert!(
        bpmux.wait_for_tui("actions", Duration::from_secs(3)),
        "palette should open"
    );
    // Type "who" to filter to the /who command, then Enter
    bpmux.type_chars("who");
    assert!(
        bpmux.wait_for_tui("/who", Duration::from_secs(5)),
        "palette should filter to /who. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_chars("\r"); // Enter
    // /who shows "Participants:" in the chat
    assert!(
        bpmux.wait_for_tui("Participants", Duration::from_secs(5)),
        "/who should have run. screen:\n{}",
        bpmux.tui()
    );
    // Palette should be closed
    bpmux
        .wait_for_tui_absent("actions", Duration::from_secs(5))
        .expect("palette should close after action");
}

/// GIVEN the palette is open
/// WHEN the user presses Esc
/// THEN the palette closes without running anything
#[test]
fn palette_esc_closes_without_action() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_ctrl('k');
    assert!(
        bpmux.wait_for_tui("actions", Duration::from_secs(3)),
        "palette should open"
    );
    bpmux.type_chars("\x1b"); // Esc
    bpmux
        .wait_for_tui_absent("actions", Duration::from_secs(5))
        .expect("palette should close after Esc");
}
