use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobRecord {
    pub name: String,
    pub conclusion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobStatusError {
    pub code: &'static str,
    pub message: String,
    pub next_action: String,
}

impl CiJobStatusError {
    pub fn invalid_id(detail: impl Into<String>) -> Self {
        Self {
            code: "invalid_id",
            message: format!("invalid run_id argument: {}", detail.into()),
            next_action: "Pass a positive integer run_id (use get_ci_state.last_run_id)"
                .to_string(),
        }
    }

    pub fn not_found(run_id: u64) -> Self {
        Self {
            code: "not_found",
            message: format!("workflow run {run_id} not found"),
            next_action: "Call get_ci_state to discover a valid last_run_id".to_string(),
        }
    }

    pub fn quota_exhausted() -> Self {
        Self {
            code: "quota_exhausted",
            message: "GitHub Actions quota exhausted for this account".to_string(),
            next_action:
                "Check spending limit at GitHub billing settings; rerun when quota refreshes"
                    .to_string(),
        }
    }

    pub fn network_error(detail: impl Into<String>) -> Self {
        Self {
            code: "network_error",
            message: format!("network/upstream failure: {}", detail.into()),
            next_action: "Retry after brief delay; check get_sync_status if persistent".to_string(),
        }
    }

    pub fn auth_failed() -> Self {
        Self {
            code: "auth_failed",
            message: "GitHub authentication failed (401/403)".to_string(),
            next_action: "Verify GITHUB_TOKEN environment / token scope (repo + workflow)"
                .to_string(),
        }
    }

    pub fn rate_limit(wait_secs: u64) -> Self {
        Self {
            code: "rate_limit",
            message: format!("GitHub API rate limit hit; retry after {wait_secs}s"),
            next_action: format!("Wait {wait_secs}s then retry"),
        }
    }

    pub fn auth_failed_with_detail(detail: &str) -> Self {
        Self {
            code: "auth_failed",
            message: format!("auth-token read failed: {detail}"),
            next_action: "Verify bpmux auth-token file is readable; restart bpmux to refresh"
                .to_string(),
        }
    }
}

pub fn compute_duration_ms(started_at: Option<&str>, completed_at: Option<&str>) -> Option<u64> {
    let start = chrono::DateTime::parse_from_rfc3339(started_at?).ok()?;
    let end = chrono::DateTime::parse_from_rfc3339(completed_at?).ok()?;
    let dur = end.signed_duration_since(start);
    let ms = dur.num_milliseconds();
    if ms >= 0 { Some(ms as u64) } else { None }
}
