//! End-to-end test for #193/#196 board view.
//!
//! Board rendering is tested with httpmock-served GitHub issues —
//! exercises the full real-substrate cascade (octocrab fetch →
//! GitHubSourceAdapter → canonical store → state-decode → renderer)
//! per Ryan's 2026-04-30 "only octocrab as test-double" directive.
//! BPMUX_BOARD_FIXTURE substrate retired here as part of #724.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::mock_gh::{issue_json, label_json};
use harness::pty::TestBpmux;
use httpmock::{Method, MockServer};
use serde_json::{Value, json};
use std::time::Duration;

fn closed_issue_json(number: u64, title: &str) -> Value {
    let mut issue = issue_json(number, title, &[], None);
    issue["state"] = json!("closed");
    issue["closed_at"] = json!("2026-04-13T00:00:00Z");
    issue
}

fn board_fixture_issues() -> Value {
    Value::Array(vec![
        issue_json(100, "Test story ready", &["ready"], Some("test body")),
        issue_json(
            101,
            "Test story delivered",
            &["delivered"],
            Some("delivered story"),
        ),
        closed_issue_json(102, "Test story accepted"),
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
            .body(json!({"workflow_runs": []}).to_string());
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
    let _ = label_json;
}

/// GIVEN the TUI is running with httpmock-served board data
/// WHEN /board is typed
/// THEN the project panel opens showing state headers + stories
/// AND chat is still visible below
/// AND Esc closes the panel
///
/// Ignored in default runs: under concurrent PTY test load the bare
/// `\x1b` sent by `type_key("Esc")` gets mis-parsed as a literal
/// escape character (echoes `^[` into chat input) instead of firing
/// crossterm's `KeyCode::Esc`. Passes reliably in isolation
/// (`cargo test --test integration_main board_view`), fails
/// intermittently in the full suite on CI + locally under load.
/// The test premise is right; the harness's Esc-timing is the flake.
/// Re-enable once the PTY harness has a reliable Esc-injection path
/// (or move to ratatui TestBackend + direct event injection like
/// `ui_tests`).
#[test]
#[ignore = "PTY harness Esc-timing flake under concurrent load; see doc above"]
fn board_renders_mocked_stories_in_panel() {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    mock_board_endpoints(&server, &board_fixture_issues());

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    let has_board = bpmux.wait_for_tui("project", Duration::from_secs(5))
        || bpmux.wait_for_tui("READY", Duration::from_secs(2))
        || bpmux.wait_for_tui("DELIVERED", Duration::from_secs(2));
    let screen = bpmux.tui();

    assert!(
        has_board,
        "board should open in project panel. screen:\n{screen}"
    );

    let has_stories =
        screen.contains("Test story") || screen.contains("#100") || screen.contains("#101");
    assert!(
        has_stories,
        "board should show mocked story data. screen:\n{screen}"
    );

    assert!(
        screen.contains(">"),
        "input prompt should be visible alongside board. screen:\n{screen}"
    );

    bpmux.type_key("Esc");
    bpmux
        .wait_until("board panel to close", Duration::from_secs(5), || {
            let s = bpmux.tui();
            !s.contains("READY (") && !s.contains("DELIVERED (") && s.contains(">")
        })
        .expect("Esc should close the board panel");
}

/// GIVEN httpmock-served stories in different states
/// WHEN the board renders
/// THEN stories appear under the correct state headers
#[test]
fn board_groups_mocked_stories_by_state() {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    mock_board_endpoints(&server, &board_fixture_issues());

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    bpmux.type_line("/board");
    assert!(
        bpmux.wait_for_tui("DELIVERED", Duration::from_secs(5)),
        "board should open and render DELIVERED header. screen:\n{}",
        bpmux.tui()
    );

    let screen = bpmux.tui();
    assert!(
        screen.contains("READY") && screen.contains("DELIVERED"),
        "board should show both state headers. screen:\n{screen}"
    );
    let lines: Vec<&str> = screen.lines().collect();
    let ready = lines
        .iter()
        .position(|l| l.contains("READY"))
        .expect("READY header missing");
    let delivered = lines
        .iter()
        .position(|l| l.contains("DELIVERED"))
        .expect("DELIVERED header missing");
    let hundred = lines.iter().position(|l| l.contains("#100"));
    let hundred_one = lines.iter().position(|l| l.contains("#101"));
    if let (Some(a), Some(b)) = (hundred, hundred_one) {
        assert!(
            ready < a && a < delivered,
            "#100 should live under READY. ready={ready}, a={a}, delivered={delivered}"
        );
        assert!(
            delivered < b,
            "#101 should live under DELIVERED. delivered={delivered}, b={b}"
        );
    }
}
