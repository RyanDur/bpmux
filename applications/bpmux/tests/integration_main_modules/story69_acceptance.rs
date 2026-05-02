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

use crate::harness;

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

/// AC3: GIVEN the fixture has stories in READY (#306) and DELIVERED
///       (#307) states
/// WHEN the board renders
/// THEN #306 appears under the READY header and #307 appears under
///      the DELIVERED header — line-position assertion so a
///      regression that swaps the grouping fails this test.
#[test]
fn ac3_board_groups_by_status() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac3", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("DELIVERED", Duration::from_secs(5)),
        "Board should show the DELIVERED column. Screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    let lines: Vec<&str> = screen.lines().collect();
    let ready_idx = lines
        .iter()
        .position(|l| l.contains("READY"))
        .expect("READY header missing");
    let delivered_idx = lines
        .iter()
        .position(|l| l.contains("DELIVERED"))
        .expect("DELIVERED header missing");
    let ready_story = lines
        .iter()
        .position(|l| l.contains("#306"))
        .expect("#306 (ready) missing from board");
    let delivered_story = lines
        .iter()
        .position(|l| l.contains("#307"))
        .expect("#307 (delivered) missing from board");
    assert!(
        ready_idx < ready_story && ready_story < delivered_idx,
        "#306 should appear after READY header and before DELIVERED \
         (ready_idx={ready_idx}, ready_story={ready_story}, \
          delivered_idx={delivered_idx}). screen:\n{screen}"
    );
    assert!(
        delivered_idx < delivered_story,
        "#307 should appear after DELIVERED header \
         (delivered_idx={delivered_idx}, delivered_story={delivered_story}). \
         screen:\n{screen}"
    );
}

/// AC4: GIVEN the board renders with fixture data
/// WHEN the human reads it
/// THEN the project panel's vertical separator line has enough
///      distinct horizontal rules to provide column structure — we
///      assert a minimum count so a plain one-border render (which
///      any ratatui Block would produce) fails the test.
#[test]
fn ac4_board_has_visual_structure() {
    let fixture = write_fixture();
    let bpmux = start_with_fixture("story69-ac4", &fixture);
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5)),
        "Board should render before structure check. Screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    // The project panel + chat panel are two adjacent bordered
    // blocks, each contributing a top and bottom border line. Plus
    // the status bar. A minimum of 4 horizontal-rule lines
    // distinguishes a real columnar layout from a single generic
    // ratatui block (which has 2).
    let rule_lines = screen
        .lines()
        .filter(|l| l.contains("─") && l.chars().filter(|c| *c == '─').count() >= 10)
        .count();
    assert!(
        rule_lines >= 4,
        "Board layout should have at least 4 long horizontal-rule \
         lines; found {rule_lines}. screen:\n{screen}"
    );
}
