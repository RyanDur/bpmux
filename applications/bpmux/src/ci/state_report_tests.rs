#![allow(clippy::unwrap_used)]

use super::state_report::*;
use crate::ci::RunEntry;
use crate::ci::{MAX_BACKOFF, POLL_INTERVAL};

fn run(status: &str, conclusion: Option<&str>, id: Option<u64>) -> RunEntry {
    RunEntry {
        status: status.to_string(),
        conclusion: conclusion.map(String::from),
        head_sha: None,
        display_title: None,
        database_id: id,
        name: None,
        annotation_message: None,
    }
}

#[test]
fn empty_runs_reports_unknown() {
    let report = build_ci_state_report(&[]);
    assert_eq!(report.conclusion, "unknown");
    assert_eq!(report.last_run_id, None);
    assert_eq!(report.url, crate::ci::github_actions_url());
}

#[test]
fn success_run_reports_success() {
    let report = build_ci_state_report(&[run("completed", Some("success"), Some(42))]);
    assert_eq!(report.conclusion, "success");
    assert_eq!(report.last_run_id, Some(42));
    assert_eq!(report.url, crate::ci::github_actions_url());
}

#[test]
fn failure_run_reports_failure() {
    let report = build_ci_state_report(&[run("completed", Some("failure"), Some(7))]);
    assert_eq!(report.conclusion, "failure");
    assert_eq!(report.last_run_id, Some(7));
}

#[test]
fn in_progress_reports_in_progress() {
    let report = build_ci_state_report(&[run("in_progress", None, Some(100))]);
    assert_eq!(report.conclusion, "in_progress");
    assert_eq!(report.last_run_id, Some(100));
}

#[test]
fn queued_reports_in_progress() {
    let report = build_ci_state_report(&[run("queued", None, Some(101))]);
    assert_eq!(report.conclusion, "in_progress");
}

#[test]
fn cancelled_reports_failure() {
    let report = build_ci_state_report(&[run("completed", Some("cancelled"), Some(5))]);
    assert_eq!(report.conclusion, "failure");
}

#[test]
fn branch_is_always_main() {
    let report = build_ci_state_report(&[]);
    assert_eq!(report.branch, "main");
}

#[test]
fn missing_run_id_points_to_actions_tab() {
    let report = build_ci_state_report(&[run("completed", Some("success"), None)]);
    assert_eq!(report.url, crate::ci::github_actions_url());
}

#[test]
fn url_is_always_all_actions_page_even_with_run_id() {
    let report = build_ci_state_report(&[run("completed", Some("failure"), Some(9999))]);
    assert_eq!(
        report.url,
        crate::ci::github_actions_url(),
        "#341 rework: URL must not embed the run_id"
    );
    assert_eq!(report.last_run_id, Some(9999));
}

#[test]
fn report_serializes_as_json_object() {
    let report = build_ci_state_report(&[run("completed", Some("success"), Some(1))]);
    let json = serde_json::to_string(&report).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["conclusion"], "success");
    assert_eq!(parsed["last_run_id"], 1);
    assert_eq!(parsed["branch"], "main");
    assert_eq!(
        parsed["url"].as_str().unwrap(),
        crate::ci::github_actions_url()
    );
    assert!(!parsed["url"].as_str().unwrap().contains("/runs/"));
}

fn run_with_annotation(conclusion: &str, msg: &str) -> RunEntry {
    RunEntry {
        status: "completed".to_string(),
        conclusion: Some(conclusion.to_string()),
        head_sha: None,
        display_title: None,
        database_id: Some(1),
        name: None,
        annotation_message: Some(msg.to_string()),
    }
}

#[test]
fn quota_exhausted_annotation_maps_to_quota_exhausted_conclusion() {
    let r = run_with_annotation(
        "failure",
        "The job was not started because recent account payments have failed or your spending limit needs to be increased.",
    );
    let report = build_ci_state_report(&[r]);
    assert_eq!(
        report.conclusion, "quota_exhausted",
        "quota-exhaustion annotation must classify as quota_exhausted, not failure"
    );
}

#[test]
fn regular_test_failure_stays_failure_not_quota_exhausted() {
    let r = run_with_annotation(
        "failure",
        "assertion failed: left != right at tests/foo.rs:42",
    );
    let report = build_ci_state_report(&[r]);
    assert_eq!(
        report.conclusion, "failure",
        "genuine test-failure must classify as failure, not quota_exhausted"
    );
}

#[test]
fn failure_with_no_annotation_stays_failure() {
    let r = run("completed", Some("failure"), Some(7));
    let report = build_ci_state_report(&[r]);
    assert_eq!(report.conclusion, "failure");
}

#[test]
fn report_carries_poll_interval_and_backoff_cap_matching_ci_watcher_constants() {
    let report = build_ci_state_report(&[run("completed", Some("success"), Some(1))]);
    assert_eq!(
        report.poll_interval_secs,
        POLL_INTERVAL.as_secs(),
        "report.poll_interval_secs must equal ci_watcher::POLL_INTERVAL — contract can't drift"
    );
    assert_eq!(
        report.backoff_cap_secs,
        MAX_BACKOFF.as_secs(),
        "report.backoff_cap_secs must equal ci_watcher::MAX_BACKOFF — contract can't drift"
    );
}

#[test]
fn unknown_report_carries_same_contract_bounds_as_built_report() {
    let unknown = unknown_ci_state_report();
    assert_eq!(unknown.poll_interval_secs, POLL_INTERVAL.as_secs());
    assert_eq!(unknown.backoff_cap_secs, MAX_BACKOFF.as_secs());
}

#[test]
fn spending_limit_phrase_alone_also_triggers_quota() {
    let r = run_with_annotation(
        "failure",
        "This repository's workflow runs are paused: spending limit reached.",
    );
    let report = build_ci_state_report(&[r]);
    assert_eq!(report.conclusion, "quota_exhausted");
}
