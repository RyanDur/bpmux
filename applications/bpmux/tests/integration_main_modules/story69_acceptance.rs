//! Story 69 Acceptance Test: A human can see project progress at a glance
//! from the TUI via `/board`.
//!
//! AC1: /board renders a compact Kanban-style view
//! AC2: Board shows issue numbers and titles
//! AC3: Board groups by status (READY, DELIVERED, ICEBOX, ...)
//! AC4: Column headers and separators provide visual structure
//!
//! Tests drive the TUI through httpmock-served GitHub issues — exercises
//! the full real-substrate cascade (octocrab → GitHubSourceAdapter →
//! canonical store → state-decode → renderer) per Ryan's 2026-04-30
//! "only octocrab as test-double" directive. BPMUX_BOARD_FIXTURE retired
//! here as part of #724 sub-slice 2.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::mock_gh::issue_json;
use harness::pty::TestBpmux;
use httpmock::{Method, MockServer};
use serde_json::Value;
use std::time::Duration;

fn board_fixture_issues() -> Value {
    Value::Array(vec![
        issue_json(306, "Ready story", &["ready"], Some("")),
        issue_json(307, "Delivered story", &["delivered"], Some("")),
        issue_json(308, "Icebox story", &[], Some("")),
    ])
}

fn mock_board_endpoints(server: &MockServer, issues: &Value) {
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(issues.to_string());
    });
    server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/actions/runs");
        then.status(200)
            .header("content-type", "application/json")
            .body(serde_json::json!({"workflow_runs": []}).to_string());
    });
    if let Value::Array(arr) = issues {
        for issue in arr {
            let number = issue["number"].as_u64().expect("issue number");
            let issue_path = format!("/repos/acme/bpmux/issues/{number}");
            let comments_path = format!("/repos/acme/bpmux/issues/{number}/comments");
            let issue_clone = issue.clone();
            server.mock(|when, then| {
                when.method(Method::GET).path(issue_path);
                then.status(200)
                    .header("content-type", "application/json")
                    .body(issue_clone.to_string());
            });
            server.mock(|when, then| {
                when.method(Method::GET).path(comments_path);
                then.status(200)
                    .header("content-type", "application/json")
                    .body(Value::Array(vec![]).to_string());
            });
        }
    }
}

fn start_with_mocked_board() -> TestBpmux {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    mock_board_endpoints(&server, &board_fixture_issues());
    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    std::mem::forget(server);
    bpmux
}

/// AC1: GIVEN the board has mocked stories
/// WHEN the human types `/board`
/// THEN the READY column header renders — the observable signal that
///      the board view is live.
#[test]
fn ac1_board_renders_in_chat() {
    let bpmux = start_with_mocked_board();
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5)),
        "Board should render the READY column header. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN the board renders a story from the mock
/// WHEN the human reads it
/// THEN the issue number is visible next to the title.
#[test]
fn ac2_board_shows_issue_numbers() {
    let bpmux = start_with_mocked_board();
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("#306", Duration::from_secs(5)),
        "Board should show the mock's #306 issue number. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN the mock has stories in READY (#306) and DELIVERED
///       (#307) states
/// WHEN the board renders
/// THEN #306 appears under the READY header and #307 appears under
///      the DELIVERED header — line-position assertion so a
///      regression that swaps the grouping fails this test.
#[test]
fn ac3_board_groups_by_status() {
    let bpmux = start_with_mocked_board();
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

/// AC4: GIVEN the board renders with mock data
/// WHEN the human reads it
/// THEN the project panel's vertical separator line has enough
///      distinct horizontal rules to provide column structure — we
///      assert a minimum count so a plain one-border render (which
///      any ratatui Block would produce) fails the test.
#[test]
fn ac4_board_has_visual_structure() {
    let bpmux = start_with_mocked_board();
    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("READY", Duration::from_secs(5)),
        "Board should render before structure check. Screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
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
