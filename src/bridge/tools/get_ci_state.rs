//! Pull-based CI state for agents. Calls `gh run list --branch main
//! -L 1` via the bridge's existing `gh` helper and returns a
//! structured report. Pure `build_ci_state_report` is separated from
//! the fetch so unit tests cover shape without network.

use crate::terminal::ci::{CiState, RunEntry, classify_runs, github_actions_url};

use super::super::tools_stories::gh;

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub(crate) struct CiStateReport {
    pub conclusion: &'static str,
    pub last_run_id: Option<u64>,
    pub branch: &'static str,
    pub url: String,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
}

pub(crate) fn build_ci_state_report(runs: &[RunEntry]) -> CiStateReport {
    let event = classify_runs(runs);
    let conclusion = match event.state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        CiState::Unknown => "unknown",
    };
    let url = match event.run_id {
        Some(id) => format!("{}/runs/{id}", github_actions_url()),
        None => github_actions_url().to_string(),
    };
    CiStateReport {
        conclusion,
        last_run_id: event.run_id,
        branch: "main",
        url,
        head_sha: event.sha,
        display_title: event.title,
    }
}

pub(in crate::bridge) async fn handle_get_ci_state(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let stdout = gh(&[
        "run",
        "list",
        "-L",
        "1",
        "--branch",
        "main",
        "--json",
        "status,conclusion,headSha,displayTitle,databaseId",
    ])
    .await?;
    let runs: Vec<RunEntry> =
        serde_json::from_str(&stdout).map_err(|e| format!("parse gh output: {e}"))?;
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
        assert_eq!(
            report.url,
            "https://github.com/RyanDur/bpmux/actions/runs/42"
        );
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

    #[test]
    fn report_serializes_as_json_object() {
        let report = build_ci_state_report(&[run("completed", Some("success"), Some(1))]);
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["conclusion"], "success");
        assert_eq!(parsed["last_run_id"], 1);
        assert_eq!(parsed["branch"], "main");
        assert!(parsed["url"].as_str().unwrap().contains("/runs/1"));
    }

    // Real-substrate boundary test — exercises gh subprocess + JSON
    // parse end-to-end. Feature-gated so default CI stays offline.
    // Run with `cargo test --features integration_tests`.
    #[cfg(feature = "integration_tests")]
    #[tokio::test]
    async fn handle_returns_structured_ci_state() {
        let args = serde_json::Map::new();
        let result = handle_get_ci_state(&args).await;
        assert!(
            result.is_ok(),
            "handle_get_ci_state should succeed with gh auth. got: {result:?}"
        );
        let json_str = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("handler should return valid JSON");
        let conclusion = parsed["conclusion"].as_str().expect("conclusion field");
        assert!(
            matches!(
                conclusion,
                "success" | "failure" | "in_progress" | "unknown"
            ),
            "conclusion should be one of the four known values, got: {conclusion}"
        );
        assert_eq!(parsed["branch"], "main");
        assert!(parsed["url"].as_str().unwrap().starts_with("https://"));
    }
}
