use crate::{CiEvent, CiState, RunEntry, WorkflowSummary, classify_runs, github_actions_url};
use crate::{MAX_BACKOFF, POLL_INTERVAL};

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
    #[serde(default)]
    pub workflows: Vec<WorkflowSummary>,
}

pub(crate) fn report_from_event(event: CiEvent) -> CiStateReport {
    let conclusion = match event.state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        CiState::QuotaExhausted => "quota_exhausted",
        CiState::Unknown => "unknown",
    };
    CiStateReport {
        conclusion,
        last_run_id: event.run_id,
        branch: "main",
        url: github_actions_url(),
        head_sha: event.sha,
        display_title: event.title,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: POLL_INTERVAL.as_secs(),
        backoff_cap_secs: MAX_BACKOFF.as_secs(),
        workflows: event.workflows,
    }
}

pub fn build_ci_state_report(runs: &[RunEntry]) -> CiStateReport {
    report_from_event(classify_runs(runs))
}

pub fn unknown_ci_state_report() -> CiStateReport {
    CiStateReport {
        conclusion: "unknown",
        last_run_id: None,
        branch: "main",
        url: github_actions_url(),
        head_sha: None,
        display_title: None,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: POLL_INTERVAL.as_secs(),
        backoff_cap_secs: MAX_BACKOFF.as_secs(),
        workflows: Vec::new(),
    }
}
