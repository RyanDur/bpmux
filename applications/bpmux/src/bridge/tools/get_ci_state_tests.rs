#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use crate::bridge::tools::build_ci_state_report;
use crate::ci::{CiState, RunEntry, classify_runs};

fn run(
    status: &str,
    conclusion: Option<&str>,
    head_sha: &str,
    name: &str,
    run_id: u64,
) -> RunEntry {
    RunEntry {
        status: status.to_string(),
        conclusion: conclusion.map(String::from),
        head_sha: Some(head_sha.to_string()),
        display_title: Some(format!("commit on {head_sha}")),
        database_id: Some(run_id),
        name: Some(name.to_string()),
        annotation_message: None,
    }
}

#[test]
fn aggregate_returns_in_progress_when_any_workflow_running() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Running,
        "if any workflow is in_progress on the latest commit, aggregate is Running"
    );
}

#[test]
fn aggregate_returns_failure_when_any_workflow_failed() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("completed", Some("failure"), "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "if any workflow failed on the latest commit, aggregate is Red"
    );
}

#[test]
fn aggregate_returns_success_only_when_all_workflows_succeeded() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("completed", Some("success"), "abc1234", "CI", 101),
        run("completed", Some("success"), "abc1234", "deploy", 102),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "all workflows green = aggregate Green"
    );
}

#[test]
fn aggregate_returns_unknown_for_no_runs() {
    let event = classify_runs(&[]);
    assert_eq!(event.state, CiState::Unknown);
    assert!(event.workflows.is_empty());
}

#[test]
fn aggregate_running_wins_over_failure() {
    let runs = vec![
        run("completed", Some("failure"), "abc1234", "CI", 100),
        run("in_progress", None, "abc1234", "smoke", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Running);
}

#[test]
fn aggregate_filters_to_latest_head_sha_ignores_older_commit_runs() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 200),
        run("completed", Some("success"), "abc1234", "CI", 201),
        run("completed", Some("failure"), "def5678", "CI", 100),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "older-commit failures don't affect aggregate of latest commit"
    );
    assert_eq!(
        event.workflows.len(),
        2,
        "only latest-commit runs in breakdown"
    );
}

#[test]
fn workflows_field_carries_per_run_breakdown() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(event.workflows.len(), 2);
    let smoke = event
        .workflows
        .iter()
        .find(|w| w.name.as_deref() == Some("smoke"))
        .expect("smoke workflow in breakdown");
    assert_eq!(smoke.conclusion, "success");
    assert_eq!(smoke.run_id, Some(100));
    let ci = event
        .workflows
        .iter()
        .find(|w| w.name.as_deref() == Some("CI"))
        .expect("CI workflow in breakdown");
    assert_eq!(ci.conclusion, "in_progress");
    assert_eq!(ci.run_id, Some(101));
}

#[test]
fn build_ci_state_report_serializes_workflows_array() {
    let runs = vec![
        run("completed", Some("success"), "abc1234", "smoke", 100),
        run("in_progress", None, "abc1234", "CI", 101),
    ];
    let report = build_ci_state_report(&runs);
    assert_eq!(report.conclusion, "in_progress");
    assert_eq!(report.workflows.len(), 2);
    let json = serde_json::to_value(&report).expect("serialize");
    assert!(
        json["workflows"].is_array(),
        "workflows must serialize as JSON array"
    );
    assert_eq!(json["workflows"].as_array().unwrap().len(), 2);
}

#[test]
fn regression_smoke_fast_main_slow_does_not_fire_premature_green() {
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "5e2f832",
            "self-hosted runner smoke-test",
            100,
        ),
        run("in_progress", None, "5e2f832", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Running,
        "smoke-green-while-CI-running must aggregate to Running (#1287 reproducer-class)"
    );
}

#[test]
fn regression_smoke_green_main_failure_aggregates_to_failure() {
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "f8c42d7",
            "self-hosted runner smoke-test",
            100,
        ),
        run("completed", Some("failure"), "f8c42d7", "CI", 101),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "smoke-green-main-failed must aggregate to Red — gate callers see failure"
    );
}

#[test]
fn single_workflow_run_aggregates_to_its_own_state() {
    let runs = vec![run("completed", Some("success"), "abc1234", "CI", 100)];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Green);
    assert_eq!(event.workflows.len(), 1);
}

#[test]
fn runs_without_head_sha_fall_back_to_single_run_classification() {
    let mut entry = run("completed", Some("success"), "", "CI", 100);
    entry.head_sha = None;
    let runs = vec![entry];
    let event = classify_runs(&runs);
    assert_eq!(event.state, CiState::Green);
}

#[test]
fn aggregate_handles_hybrid_runner_shape_post_1253_cutover() {
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "test",
            75572890367,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "build-bpmux",
            75572890351,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (1)",
            75573139951,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (2)",
            75573139874,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (3)",
            75573139941,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (4)",
            75573139917,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (5)",
            75573140045,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "playwright-e2e (6)",
            75573140101,
        ),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Green,
        "hybrid runner shape (1 self-hosted + 7 ubuntu-latest jobs) aggregates Green when all succeed"
    );
    assert_eq!(
        event.workflows.len(),
        8,
        "all 8 jobs surface in workflows[] breakdown"
    );
}

#[test]
fn aggregate_red_on_hybrid_shape_when_playwright_shard_fails() {
    let runs = vec![
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "test",
            75572890367,
        ),
        run(
            "completed",
            Some("success"),
            "57f1a08c",
            "build-bpmux",
            75572890351,
        ),
        run(
            "completed",
            Some("failure"),
            "57f1a08c",
            "playwright-e2e (2)",
            75573139874,
        ),
    ];
    let event = classify_runs(&runs);
    assert_eq!(
        event.state,
        CiState::Red,
        "a single playwright shard failure on ubuntu-latest still aggregates to Red"
    );
}

#[test]
fn workflow_summary_conclusion_uses_canonical_strings() {
    let runs = vec![
        run("completed", Some("success"), "abc", "a", 1),
        run("completed", Some("failure"), "abc", "b", 2),
        run("in_progress", None, "abc", "c", 3),
        run("queued", None, "abc", "d", 4),
    ];
    let event = classify_runs(&runs);
    let conclusions: Vec<&str> = event.workflows.iter().map(|w| w.conclusion).collect();
    assert!(conclusions.contains(&"success"));
    assert!(conclusions.contains(&"failure"));
    assert!(conclusions.iter().filter(|c| **c == "in_progress").count() >= 2);
}

fn quota_run(head_sha: &str, run_id: u64) -> RunEntry {
    RunEntry {
        status: "completed".to_string(),
        conclusion: Some("failure".to_string()),
        head_sha: Some(head_sha.to_string()),
        display_title: Some(format!("commit on {head_sha}")),
        database_id: Some(run_id),
        name: Some("CI".to_string()),
        annotation_message: Some(
            "The job was not started because recent account payments have failed or your spending limit needs to be increased."
                .to_string(),
        ),
    }
}

#[test]
fn aggregate_quota_exhausted_wins_over_green_but_loses_to_red_and_running() {
    assert_eq!(
        classify_runs(&[
            run("completed", Some("success"), "abc1234", "smoke", 201),
            quota_run("abc1234", 200),
        ])
        .state,
        CiState::QuotaExhausted,
        "quota-exhausted with no red/running run aggregates to QuotaExhausted (not Green)"
    );
    assert_eq!(
        classify_runs(&[
            run("completed", Some("failure"), "abc1234", "smoke", 202),
            quota_run("abc1234", 200),
        ])
        .state,
        CiState::Red,
        "a genuine failure outranks quota-exhausted"
    );
    assert_eq!(
        classify_runs(&[
            run("in_progress", None, "abc1234", "smoke", 203),
            quota_run("abc1234", 200),
        ])
        .state,
        CiState::Running,
        "an in-progress run outranks quota-exhausted"
    );
}
