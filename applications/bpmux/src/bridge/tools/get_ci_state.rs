use crate::ci::{CiState, RunEntry, WorkflowSummary, classify_runs, github_actions_url};
use crate::transport::ci_watcher::{MAX_BACKOFF, POLL_INTERVAL};

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

pub fn build_ci_state_report(runs: &[RunEntry]) -> CiStateReport {
    let event = classify_runs(runs);
    let conclusion = match event.state {
        CiState::Green => "success",
        CiState::Red => "failure",
        CiState::Running => "in_progress",
        CiState::QuotaExhausted => "quota_exhausted",
        CiState::Unknown => "unknown",
    };
    let url = github_actions_url();
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

pub(in crate::bridge) async fn handle_get_ci_state(
    _args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, "/api/ci");
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
