//! Pull-based CI state for agents. Fetches the most recent workflow
//! run on main via the tracker's octocrab client (#320 unified; was
//! `gh run list` subprocess pre-#320) and returns a structured
//! report. Pure `build_ci_state_report` is separated from the fetch
//! so unit tests cover shape without network.

use crate::bridge::tracker::{StoryTracker, WorkflowRunEntry};
use crate::terminal::ci::{CiState, RunEntry, classify_runs, github_actions_url};

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub struct CiStateReport {
    pub conclusion: &'static str,
    pub last_run_id: Option<u64>,
    pub branch: &'static str,
    pub url: String,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
}

pub fn build_ci_state_report(runs: &[RunEntry]) -> CiStateReport {
    let event = classify_runs(runs);
    let conclusion = match event.state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        // #397 new value — distinct from "failure" so team-gate
        // code can branch: pass-with-citation vs hard-block.
        CiState::QuotaExhausted => "quota_exhausted",
        CiState::Unknown => "unknown",
    };
    // #341 rework: the click-through URL is always the repo's top-level
    // Actions page, never the run-specific URL. Ryan 2026-04-22 ~22:52
    // UTC: "CI used to take me to all the actions." The run-specific
    // URL cost him the overview context when he was triaging state
    // transitions; the all-actions page gives him run history + the
    // latest run at the top. `last_run_id` stays in the payload so
    // future callers that want the run-specific URL can synthesize
    // it themselves.
    let url = github_actions_url().to_string();
    CiStateReport {
        conclusion,
        last_run_id: event.run_id,
        branch: "main",
        url,
        head_sha: event.sha,
        display_title: event.title,
    }
}

/// Convert the tracker's generic `WorkflowRunEntry` shape into the
/// `RunEntry` shape `terminal::ci::classify_runs` consumes. The two
/// structs differ only in field visibility (tracker struct is public
/// for cross-module reuse; RunEntry is crate-internal classification
/// type). One-to-one field copy including #397's annotation_message
/// so quota-exhaustion classification sees the reason from GitHub.
fn run_entry_from_tracker(w: &WorkflowRunEntry) -> RunEntry {
    RunEntry {
        status: w.status.clone(),
        conclusion: w.conclusion.clone(),
        head_sha: w.head_sha.clone(),
        display_title: w.display_title.clone(),
        database_id: w.database_id,
        annotation_message: w.annotation_message.clone(),
    }
}

pub(in crate::bridge) async fn handle_get_ci_state(
    _args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let latest = tracker.list_latest_run_on_main().await?;
    let runs: Vec<RunEntry> = latest.iter().map(run_entry_from_tracker).collect();
    let report = build_ci_state_report(&runs);
    serde_json::to_string_pretty(&report).map_err(|e| format!("serialize: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn run(status: &str, conclusion: Option<&str>, id: Option<u64>) -> RunEntry {
        RunEntry {
            status: status.to_string(),
            conclusion: conclusion.map(String::from),
            head_sha: None,
            display_title: None,
            database_id: id,
            annotation_message: None,
        }
    }

    #[test]
    fn empty_runs_reports_unknown() {
        let report = build_ci_state_report(&[]);
        assert_eq!(report.conclusion, "unknown");
        assert_eq!(report.last_run_id, None);
        assert_eq!(report.url, "https://github.com/RyanDur/bpmux/actions");
    }

    #[test]
    fn success_run_reports_success() {
        let report = build_ci_state_report(&[run("completed", Some("success"), Some(42))]);
        assert_eq!(report.conclusion, "success");
        assert_eq!(report.last_run_id, Some(42));
        // #341 rework: URL is always the all-actions page, not
        // /runs/{id}. `last_run_id` stays in the payload for callers
        // that want to synthesize the run-specific URL themselves.
        assert_eq!(report.url, "https://github.com/RyanDur/bpmux/actions");
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
        assert_eq!(report.url, "https://github.com/RyanDur/bpmux/actions");
    }

    // #341 rework falsifier guard: URL must be the all-actions page
    // regardless of run_id presence. If a future refactor re-introduces
    // the run-specific URL, this test fails with a clear diff on the
    // expected URL shape.
    #[test]
    fn url_is_always_all_actions_page_even_with_run_id() {
        let report = build_ci_state_report(&[run("completed", Some("failure"), Some(9999))]);
        assert_eq!(
            report.url, "https://github.com/RyanDur/bpmux/actions",
            "#341 rework: URL must not embed the run_id"
        );
        // last_run_id preserved for callers that want the run-specific URL
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
        // #341 rework: URL field carries the all-actions page even
        // when a run_id is present. last_run_id stays in the payload
        // separately so it can be used elsewhere.
        assert!(parsed["url"].as_str().unwrap().ends_with("/actions"));
        assert!(!parsed["url"].as_str().unwrap().contains("/runs/"));
    }

    #[test]
    fn run_entry_from_tracker_preserves_fields() {
        let w = WorkflowRunEntry {
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            head_sha: Some("abc123".to_string()),
            display_title: Some("CI: fix bug".to_string()),
            database_id: Some(99),
            annotation_message: None,
        };
        let r = run_entry_from_tracker(&w);
        assert_eq!(r.status, "completed");
        assert_eq!(r.conclusion.as_deref(), Some("success"));
        assert_eq!(r.head_sha.as_deref(), Some("abc123"));
        assert_eq!(r.display_title.as_deref(), Some("CI: fix bug"));
        assert_eq!(r.database_id, Some(99));
        assert_eq!(r.annotation_message, None);
    }

    // ── #397 quota-exhausted classifier tests ──

    fn run_with_annotation(conclusion: &str, msg: &str) -> RunEntry {
        RunEntry {
            status: "completed".to_string(),
            conclusion: Some(conclusion.to_string()),
            head_sha: None,
            display_title: None,
            database_id: Some(1),
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
        // A genuine test-failure annotation (e.g., an assertion
        // error) should NOT trip the quota classifier.
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
        // Failed run without any annotation = legacy path; treat as
        // failure (pre-#397 behavior preserved when annotation
        // fetch returns nothing).
        let r = run("completed", Some("failure"), Some(7));
        let report = build_ci_state_report(&[r]);
        assert_eq!(report.conclusion, "failure");
    }

    #[test]
    fn spending_limit_phrase_alone_also_triggers_quota() {
        // Symmetric coverage: the annotation's other canonical
        // phrases should also match.
        let r = run_with_annotation(
            "failure",
            "This repository's workflow runs are paused: spending limit reached.",
        );
        let report = build_ci_state_report(&[r]);
        assert_eq!(report.conclusion, "quota_exhausted");
    }
}
