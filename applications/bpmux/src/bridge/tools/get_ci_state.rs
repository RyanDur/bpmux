//! Pull-based CI state for agents. Fetches the most recent workflow
//! run on main via the tracker's octocrab client (#320 unified; was
//! `gh run list` subprocess pre-#320) and returns a structured
//! report. Pure `build_ci_state_report` is separated from the fetch
//! so unit tests cover shape without network.

use crate::ci::{CiState, RunEntry, WorkflowSummary, classify_runs, github_actions_url};
use crate::transport::ci_watcher::{MAX_BACKOFF, POLL_INTERVAL};

/// MCP `get_ci_state` response shape.
///
/// **Aggregate contract (#1287)**: `conclusion` aggregates the
/// states of ALL workflow runs triggered by the latest commit (the
/// head_sha of the most-recent run). It is NOT the conclusion of
/// the last-workflow-to-conclude. Specifically:
/// - `success` IFF all runs on the latest commit concluded with success
/// - `failure` IF any run concluded with failure/cancelled (non-quota)
/// - `quota_exhausted` IF any run is quota-blocked and none are red/running
/// - `in_progress` IF any run on the latest commit is still running/queued
/// - `unknown` if no runs are visible on the latest commit
///
/// `workflows` carries the per-run breakdown so callers needing
/// per-workflow detail don't have to re-fetch via
/// `get_ci_log_for_run`. `last_run_id` remains the most-recent run's
/// id (legacy field; specific run lookup belongs to
/// `get_ci_log_for_run`).
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct CiStateReport {
    pub conclusion: &'static str,
    pub last_run_id: Option<u64>,
    pub branch: &'static str,
    pub url: String,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    #[serde(default)]
    pub last_updated_at: Option<String>,
    #[serde(default)]
    pub last_successful_poll_at: Option<String>,
    pub poll_interval_secs: u64,
    pub backoff_cap_secs: u64,
    /// #1287: per-workflow run summaries on the latest commit.
    /// Each entry: `{name, run_id, conclusion}`. `conclusion`
    /// strings follow the same vocabulary as the top-level
    /// aggregate (`success` / `failure` / `in_progress` /
    /// `quota_exhausted` / `unknown`).
    #[serde(default)]
    pub workflows: Vec<WorkflowSummary>,
}

pub fn build_ci_state_report(runs: &[RunEntry]) -> CiStateReport {
    let event = classify_runs(runs);
    let conclusion = match event.state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        CiState::QuotaExhausted => "quota_exhausted",
        CiState::Unknown => "unknown",
    };
    let url = github_actions_url().to_string();
    CiStateReport {
        conclusion,
        last_run_id: event.run_id,
        branch: "main",
        url,
        head_sha: event.sha,
        display_title: event.title,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: POLL_INTERVAL.as_secs(),
        backoff_cap_secs: MAX_BACKOFF.as_secs(),
        workflows: event.workflows,
    }
}

pub fn unknown_ci_state_report() -> CiStateReport {
    CiStateReport {
        conclusion: "unknown",
        last_run_id: None,
        branch: "main",
        url: github_actions_url().to_string(),
        head_sha: None,
        display_title: None,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: POLL_INTERVAL.as_secs(),
        backoff_cap_secs: MAX_BACKOFF.as_secs(),
        workflows: Vec::new(),
    }
}

/// #526: bridges read CI state from the electron-server's canonical
/// `/api/ci` endpoint instead of holding their own GitHubApi handle.
/// One ci_watcher on the server fetches GH; every bridge renders the
/// same report via this RPC. No more per-bridge rate-limit accounting
/// or momentary cross-bridge divergence during fetch windows.
pub(in crate::bridge) async fn handle_get_ci_state(
    _args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = format!("http://127.0.0.1:{port}/api/ci");
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("get_ci_state: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("get_ci_state: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("get_ci_state: {status} — {body}"));
    }
    Ok(body)
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
            name: None,
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

    // ── #397 quota-exhausted classifier tests ──

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
