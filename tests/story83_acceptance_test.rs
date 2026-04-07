//! Story 83 Acceptance Test: A user can view the project roadmap from the TUI.
//!
//! AC1: /roadmap displays ROADMAP.md contents
//! AC2: The roadmap renders with markdown formatting
//! AC3: The display is scrollable (long message behavior)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN the user types /roadmap
/// WHEN ROADMAP.md exists in the repo
/// THEN the roadmap content appears in the chat
#[test]
fn ac1_roadmap_displays_content() {
    let bpmux = TestBpmux::start();

    bpmux.type_line("/roadmap");
    std::thread::sleep(Duration::from_millis(500));

    // ROADMAP.md heading should appear in the chat area
    assert!(
        bpmux.wait_for_tui("bpmux Roadmap", Duration::from_secs(3)),
        "Roadmap content should appear. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN /roadmap displays a long document
/// WHEN the content exceeds the viewport
/// THEN truncation indicator shows and Enter expands it
#[test]
fn ac3_roadmap_is_expandable() {
    let bpmux = TestBpmux::start();

    bpmux.type_line("/roadmap");

    // Long roadmap should be truncated with a line count indicator
    assert!(
        bpmux.wait_for_tui("lines]", Duration::from_secs(3)),
        "Long roadmap should show truncation indicator. Screen:\n{}",
        bpmux.tui()
    );

    // Press Enter on empty input to expand the last (truncated) message
    bpmux.type_line("");
    std::thread::sleep(Duration::from_secs(2));

    // After expanding, the truncation indicator should be gone
    let screen = bpmux.tui();
    assert!(
        !screen.contains("lines]"),
        "Expanded roadmap should not show truncation indicator. Screen:\n{screen}"
    );
}
