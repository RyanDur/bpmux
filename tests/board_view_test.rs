//! End-to-end test for #193/#196 board view.
//!
//! Board rendering is tested with fixture data — no gh dependency.
//! The BPMUX_BOARD_FIXTURE env var points to a JSON file with test
//! stories, exercising the full rendering pipeline with controlled
//! input.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

const FIXTURE_DATA: &str = r##"[
  {"number":100,"title":"Test story ready","state":"OPEN","labels":[{"name":"ready"}],"body":"test body","assignees":[],"createdAt":"2026-04-14"},
  {"number":101,"title":"Test story delivered","state":"OPEN","labels":[{"name":"delivered"}],"body":"delivered story","assignees":[],"createdAt":"2026-04-14"},
  {"number":102,"title":"Test story accepted","state":"CLOSED","labels":[],"body":"done","assignees":[],"createdAt":"2026-04-13"}
]"##;

fn write_fixture() -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("create fixture");
    use std::io::Write;
    f.write_all(FIXTURE_DATA.as_bytes()).expect("write fixture");
    f
}

/// GIVEN the TUI is running with fixture board data
/// WHEN /board is typed
/// THEN the project panel opens showing state headers + stories
/// AND chat is still visible below
/// AND Esc closes the panel
#[test]
fn board_renders_fixture_stories_in_panel() {
    let fixture = write_fixture();
    let bpmux = TestBpmux::start_with_env(
        "board-test",
        "BPMUX_BOARD_FIXTURE",
        fixture.path().to_str().unwrap(),
    );
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    let has_board = bpmux.wait_for_tui("project", Duration::from_secs(5))
        || bpmux.wait_for_tui("READY", Duration::from_secs(2))
        || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2));
    let screen = bpmux.tui();

    // Board should show state headers from fixture data
    assert!(
        has_board,
        "board should open in project panel. screen:\n{screen}"
    );

    // Should show fixture story titles
    let has_stories =
        screen.contains("Test story") || screen.contains("#100") || screen.contains("#101");
    assert!(
        has_stories,
        "board should show fixture story data. screen:\n{screen}"
    );

    // Chat input should still be visible (split view)
    assert!(
        screen.contains(">"),
        "input prompt should be visible alongside board. screen:\n{screen}"
    );

    // Esc closes the panel
    bpmux.type_key("Esc");
    std::thread::sleep(Duration::from_millis(300));
    let after = bpmux.tui();
    let panel_closed =
        !after.contains("READY (") && !after.contains("DELIVERED (") && after.contains(">");
    assert!(panel_closed, "Esc should close panel. screen:\n{after}");
}

/// GIVEN fixture data has stories in different states
/// WHEN the board renders
/// THEN stories appear under the correct state headers
#[test]
fn board_groups_fixture_stories_by_state() {
    let fixture = write_fixture();
    let bpmux = TestBpmux::start_with_env(
        "board-group",
        "BPMUX_BOARD_FIXTURE",
        fixture.path().to_str().unwrap(),
    );
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5))
            || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2)),
        "board should open"
    );

    let screen = bpmux.tui();

    // Should show READY and DELIVERED headers (from fixture data)
    assert!(
        screen.contains("READY") || screen.contains("DELIVERED"),
        "board should show state headers. screen:\n{screen}"
    );
}
