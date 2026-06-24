#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiTestFailure {
    pub shard_id: Option<String>,
    pub test_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub error_message: String,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiRunFailures {
    pub run_id: u64,
    pub conclusion: String,
    pub failures: Vec<CiTestFailure>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CiLogError {
    RunNotFound { run_id: u64 },
    RunInProgress { run_id: u64, conclusion: String },
    RateLimited { wait_secs: u64 },
    UpstreamError { detail: String },
}

pub(in crate::bridge) async fn handle_get_ci_log_for_run(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'run_id' argument")?;
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/ci/log/{run_id}"));
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("get_ci_log_for_run: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("get_ci_log_for_run: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("get_ci_log_for_run: {status} — {body}"));
    }
    Ok(body)
}

#[cfg(test)]
use super::ci_log_parse::{build_non_test_failure_record, parse_playwright_failures_from_log};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "get_ci_log_for_run_tests.rs"]
mod tests;
