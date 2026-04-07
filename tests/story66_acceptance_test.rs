//! Story 66 Acceptance Test: Project node in sidebar tree view.
//!
//! AC1: Channels appear indented under a project root node
//! AC2: Left arrow collapses project node
//! AC3: Right arrow expands project node
//! AC4: Project node expanded by default
//! AC5: Deselected state (project node) shows all messages

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1+AC4: GIVEN the sidebar is visible and project is expanded by default
/// WHEN the user looks at the channel list
/// THEN channels appear indented under a project root node with ▾
#[test]
fn ac1_ac4_channels_under_project_node() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("? help", Duration::from_secs(5));
    // Open sidebar with Ctrl+B
    bpmux.type_ctrl('b');
    std::thread::sleep(Duration::from_millis(300));
    let screen = bpmux.tui();
    // Should show the expand indicator ▾ for the project node
    assert!(
        screen.contains('▾'),
        "Should show expanded project node with ▾. Screen:\n{}",
        screen
    );
}

/// AC2: GIVEN the project node is expanded
/// WHEN the user presses Left arrow on the project node
/// THEN it collapses, hiding its channels
#[test]
fn ac2_left_collapses_project() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("? help", Duration::from_secs(5));
    // Send a message to create a channel
    bpmux.type_line("hello");
    std::thread::sleep(Duration::from_millis(300));
    // Open sidebar
    bpmux.type_ctrl('b');
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        bpmux.tui().contains('▾'),
        "Should start expanded. Screen:\n{}",
        bpmux.tui()
    );
    // Press Left to collapse
    bpmux.type_key("Left");
    std::thread::sleep(Duration::from_millis(300));
    let screen = bpmux.tui();
    assert!(
        screen.contains('▸'),
        "Should show collapsed indicator ▸. Screen:\n{}",
        screen
    );
}

/// AC3: GIVEN the project node is collapsed
/// WHEN the user presses Right arrow on it
/// THEN it expands, showing its channels
#[test]
fn ac3_right_expands_project() {
    let bpmux = TestBpmux::start();
    bpmux.wait_for_tui("? help", Duration::from_secs(5));
    // Open sidebar and collapse
    bpmux.type_ctrl('b');
    std::thread::sleep(Duration::from_millis(300));
    bpmux.type_key("Left");
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        bpmux.tui().contains('▸'),
        "Should be collapsed. Screen:\n{}",
        bpmux.tui()
    );
    // Press Right to expand
    bpmux.type_key("Right");
    std::thread::sleep(Duration::from_millis(300));
    let screen = bpmux.tui();
    assert!(
        screen.contains('▾'),
        "Should show expanded indicator ▾. Screen:\n{}",
        screen
    );
}
