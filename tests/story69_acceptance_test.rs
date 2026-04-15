//! Story 69 Acceptance Test: A human can see project progress at a glance
//! from the TUI via `/board`.
//!
//! AC1: /board renders a compact Kanban-style view
//! AC2: Board shows issue numbers and titles
//! AC3: Board groups by status (READY, DELIVERED, ICEBOX, ...)
//! AC4: Column headers and separators provide visual structure
//!
//! Tests drive the TUI through a fixture (`BPMUX_BOARD_FIXTURE`) rather
//! than shelling out to `gh`. The fixture path is already wired into
//! `src/terminal/board.rs`; the same env var is used by the agent-side
//! bridge (`handle_view_board`) so both audiences read from the same
//! shape. Hermetic by construction — no silent skips when gh or auth
//! is unavailable.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::io::Write;
use std::time::Duration;

/// Shared fixture: one story per state category so every AC has a
/// deterministic signal to assert on (no `||` alternates, no
/// probing). The JSON shape matches what `gh issue list --json
/// number,title,state,labels,...` produces in production, so the
/// same parsing path exercises the fixture and live data.
const BOARD_FIXTURE: &str = r#"[
  {"number":306,"title":"Ready story","state":"OPEN","labels":[{"name":"ready"}],"body":"","assignees":[],"createdAt":"2026-04-14"},
  {"number":307,"title":"Delivered story","state":"OPEN","labels":[{"name":"delivered"}],"body":"","assignees":[],"createdAt":"2026-04-14"},
  {"number":308,"title":"Icebox story","state":"OPEN","labels":[],"body":"","assignees":[],"createdAt":"2026-04-13"}
]"#;

fn write_fixture() -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("fixture file");
    f.write_all(BOARD_FIXTURE.as_bytes())
        .expect("write fixture");
    f
}

fn start_with_fixture(name: &str, fixture: &tempfile::NamedTempFile) -> TestBpmux {
    let path = fixture.path().to_str().expect("fixture path utf-8");
    TestBpmux::start_with_env(name, "BPMUX_BOARD_FIXTURE", path)
}

/// AC1: GIVEN the board has fixture stories
/// WHEN the human types `/board`
/// THEN the READY column header renders — the observable signal that
///      the board view is live.
#[test]
fn ac1_board_renders_in_chat() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac1", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5)),
        "Board should render the READY column header. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN the board renders a story from the fixture
/// WHEN the human reads it
/// THEN the issue number is visible next to the title.
#[test]
fn ac2_board_shows_issue_numbers() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac2", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("#306", Duration::from_secs(5)),
        "Board should show the fixture's #306 issue number. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN the fixture has stories in READY and DELIVERED states
/// WHEN the board renders
/// THEN both state columns appear — the fixture guarantees both are
///      present, so this is a deterministic two-signal assertion.
#[test]
fn ac3_board_groups_by_status() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac3", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5)),
        "Board should show the READY column. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("DELIVERED", Duration::from_secs(5)),
        "Board should show the DELIVERED column. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN the board renders with fixture data
/// WHEN the human reads it
/// THEN separator lines provide visual structure.
#[test]
fn ac4_board_has_visual_structure() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac4", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("─", Duration::from_secs(5)),
        "Board should have separator lines (─). Screen:\n{}",
        bpmux.tui()
    );
}
