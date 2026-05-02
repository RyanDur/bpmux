//! Story 66 Acceptance Test: Project node in sidebar tree view.
//!
//! AC1: Channels appear indented under a project root node
//! AC2: Left arrow collapses project node
//! AC3: Right arrow expands project node
//! AC4: Project node expanded by default
//! AC5: Deselected state (project node) shows all messages

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

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
    assert!(
        bpmux.wait_for_tui("▾", Duration::from_secs(5)),
        "Should show expanded project node with ▾. Screen:\n{}",
        bpmux.tui()
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
    assert!(
        bpmux.wait_for_tui("hello", Duration::from_secs(5)),
        "message should appear before opening sidebar. screen:\n{}",
        bpmux.tui()
    );
    // Open sidebar
    bpmux.type_ctrl('b');
    assert!(
        bpmux.wait_for_tui("▾", Duration::from_secs(5)),
        "Should start expanded. Screen:\n{}",
        bpmux.tui()
    );
    // Press Left to collapse
    bpmux.type_key("Left");
    assert!(
        bpmux.wait_for_tui("▸", Duration::from_secs(5)),
        "Should show collapsed indicator ▸. Screen:\n{}",
        bpmux.tui()
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
    assert!(
        bpmux.wait_for_tui("▾", Duration::from_secs(5)),
        "sidebar should start expanded. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_key("Left");
    assert!(
        bpmux.wait_for_tui("▸", Duration::from_secs(5)),
        "Should be collapsed. Screen:\n{}",
        bpmux.tui()
    );
    // Press Right to expand
    bpmux.type_key("Right");
    assert!(
        bpmux.wait_for_tui("▾", Duration::from_secs(5)),
        "Should show expanded indicator ▾. Screen:\n{}",
        bpmux.tui()
    );
}
