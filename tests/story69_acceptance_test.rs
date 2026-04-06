//! Story 69 Acceptance Test: A human can see project progress at a glance
//! from the TUI via `/board`.
//!
//! AC1: /board renders a compact Kanban-style view
//! AC2: Board shows issue numbers and titles
//! AC3: Board groups by status (Active, Backlog, Icebox, Done)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Detect the GitHub repo from git remote and set BPMUX_REPO env var.
/// Returns false if gh or git remote detection fails.
fn ensure_board_env() -> bool {
    if std::process::Command::new("gh")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("SKIP: gh CLI not available");
        return false;
    }
    if std::env::var("BPMUX_REPO").is_ok() {
        return true;
    }
    // Detect repo from git remote
    if let Ok(output) = std::process::Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ])
        .output()
        && output.status.success()
    {
        let repo = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !repo.is_empty() {
            // SAFETY: Tests run sequentially in this binary; no other thread reads this var
            unsafe {
                std::env::set_var("BPMUX_REPO", &repo);
            }
            return true;
        }
    }
    eprintln!("SKIP: could not detect GitHub repo");
    false
}

/// AC1: GIVEN `gh` CLI is available
/// WHEN the human types `/board`
/// THEN a board view renders in the chat area
#[test]
fn ac1_board_renders_in_chat() {
    if !ensure_board_env() {
        return;
    }

    let bpmux = TestBpmux::start();
    bpmux.type_line("/board");
    // Board may scroll off top of viewport; check for section markers visible in view
    let has_board = bpmux.wait_for_tui("Active", Duration::from_secs(20))
        || bpmux.wait_for_tui("Backlog", Duration::from_secs(2))
        || bpmux.wait_for_tui("Icebox", Duration::from_secs(2));
    assert!(has_board, "Should show board. Screen:\n{}", bpmux.tui());
}

/// AC2: GIVEN the board renders
/// WHEN the human reads it
/// THEN issue numbers are visible
#[test]
fn ac2_board_shows_issue_numbers() {
    if !ensure_board_env() {
        return;
    }

    let bpmux = TestBpmux::start();
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("#", Duration::from_secs(20)),
        "Board should show issue numbers. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN issues exist in various states
/// WHEN the board renders
/// THEN it groups them with section markers (●, ○, ◌)
#[test]
fn ac3_board_groups_by_status() {
    if !ensure_board_env() {
        return;
    }

    let bpmux = TestBpmux::start();
    bpmux.type_line("/board");
    let has_section = bpmux.wait_for_tui("Active", Duration::from_secs(20))
        || bpmux.wait_for_tui("Backlog", Duration::from_secs(2))
        || bpmux.wait_for_tui("Icebox", Duration::from_secs(2));
    assert!(
        has_section,
        "Board should show status sections. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN the board renders
/// WHEN the human reads it
/// THEN section markers provide visual structure
#[test]
fn ac4_board_has_visual_structure() {
    if !ensure_board_env() {
        return;
    }

    let bpmux = TestBpmux::start();
    bpmux.type_line("/board");
    // Section markers: ● Active, ○ Backlog, ◌ Icebox
    let has_marker = bpmux.wait_for_tui("●", Duration::from_secs(20))
        || bpmux.wait_for_tui("○", Duration::from_secs(2))
        || bpmux.wait_for_tui("◌", Duration::from_secs(2));
    assert!(
        has_marker,
        "Board should have section markers. Screen:\n{}",
        bpmux.tui()
    );
}
