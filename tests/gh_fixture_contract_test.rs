//! Contract tests for the gh CLI subprocess boundary.
//!
//! Complements the octocrab httpmock contract suite. The octocrab
//! path is covered by `src/bridge/tracker_contract_tests.rs`; this
//! file locks down the SHAPE of gh's JSON output that bpmux parses:
//! `gh issue list --json number,title,state,labels` and
//! `gh run list --json status,conclusion,headSha,displayTitle,databaseId`.
//!
//! Fixtures live in `tests/fixtures/gh/` and are checked into the
//! repo. When real gh changes a field name or drops a key, the
//! feature-gated integration suite (#299) surfaces the drift and
//! `scripts/rerecord-fixtures.sh` regenerates these files from the
//! live provider. This suite then stays offline-reproducible for
//! pre-push + CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str) -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "gh", name]
        .iter()
        .collect();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse fixture {path:?}: {e}"))
}

#[test]
fn issue_list_fixture_has_fields_board_view_depends_on() {
    // Contract: `gh issue list --json number,title,state,labels` returns
    // an array of objects with these exact top-level keys. Drift in any
    // key would silently break the board view's story grouping.
    let fixture = fixture("issue_list_open.json");
    let arr = fixture.as_array().expect("list is an array");
    assert!(!arr.is_empty(), "fixture must carry at least one issue");
    for entry in arr {
        assert!(
            entry.get("number").and_then(Value::as_u64).is_some(),
            "each entry needs numeric `number`: {entry}"
        );
        assert!(
            entry.get("title").and_then(Value::as_str).is_some(),
            "each entry needs string `title`: {entry}"
        );
        let state = entry
            .get("state")
            .and_then(Value::as_str)
            .expect("state is a string");
        assert!(
            state == "OPEN" || state == "CLOSED",
            "state must be OPEN/CLOSED: {state}"
        );
        let labels = entry
            .get("labels")
            .and_then(Value::as_array)
            .expect("labels is an array");
        for label in labels {
            assert!(
                label.get("name").and_then(Value::as_str).is_some(),
                "each label needs `name`: {label}"
            );
        }
    }
}

#[test]
fn run_list_success_fixture_parses_to_green_ci_state() {
    let fixture = fixture("run_list_success.json");
    let arr = fixture.as_array().expect("list is an array");
    assert_eq!(arr.len(), 1, "fixture captures a single run");
    let run = &arr[0];
    assert_eq!(run["status"], "completed");
    assert_eq!(run["conclusion"], "success");
    assert!(run.get("headSha").and_then(Value::as_str).is_some());
    assert!(run.get("displayTitle").and_then(Value::as_str).is_some());
    assert!(run.get("databaseId").and_then(Value::as_u64).is_some());
}

#[test]
fn run_list_failure_fixture_parses_to_red_ci_state() {
    let fixture = fixture("run_list_failure.json");
    let arr = fixture.as_array().expect("list is an array");
    let run = &arr[0];
    assert_eq!(run["status"], "completed");
    assert_eq!(run["conclusion"], "failure");
}

/// Awareness signal (#299 pattern, extended in #304): always-visible
/// pass confirms the contract suite is wired in. If the fixture
/// files go missing, the tests above fail loudly; if the test
/// harness itself breaks, this single-line test surfaces it too.
#[test]
fn contract_tests_run_on_every_cargo_test_no_credentials_needed() {
    // The octocrab httpmock suite + gh fixture suite are regular
    // `#[test]`s — they run on every `cargo test` without any env
    // vars or feature flags. Pre-push + CI auto-invoke them.
    assert!(env!("CARGO_MANIFEST_DIR").contains("bpmux"));
}
