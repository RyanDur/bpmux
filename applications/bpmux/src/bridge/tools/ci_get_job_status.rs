use serde::Serialize;

pub const MAX_DEFAULT_BUDGET_BYTES: usize = 2048;
const TRUNCATION_HINT: &str = "set verbose=true for full payload";

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
pub struct CiJobTruncation {
    pub shown: usize,
    pub total: usize,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobStatusData {
    pub run_id: u64,
    pub jobs: Vec<CiJobRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<CiJobTruncation>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobStatusResponse {
    pub mutation_status: &'static str,
    pub data: CiJobStatusData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action_hint: Option<String>,
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
}

pub fn build_ci_job_status_response(
    run_id: u64,
    jobs: Vec<CiJobRecord>,
    verbose: bool,
) -> CiJobStatusResponse {
    let total = jobs.len();
    if verbose {
        return ok_response(CiJobStatusData {
            run_id,
            jobs,
            truncated: None,
        });
    }
    let envelope_overhead = estimate_envelope_overhead(run_id, total);
    let mut accumulated = envelope_overhead;
    let mut keep = 0;
    for job in &jobs {
        let job_size = serde_json::to_string(job).map(|s| s.len() + 1).unwrap_or(0);
        if keep > 0 && accumulated + job_size > MAX_DEFAULT_BUDGET_BYTES {
            break;
        }
        accumulated += job_size;
        keep += 1;
    }
    if keep >= total {
        ok_response(CiJobStatusData {
            run_id,
            jobs,
            truncated: None,
        })
    } else {
        let kept = keep.max(1);
        let kept_jobs: Vec<CiJobRecord> = jobs.into_iter().take(kept).collect();
        ok_response(CiJobStatusData {
            run_id,
            jobs: kept_jobs,
            truncated: Some(CiJobTruncation {
                shown: kept,
                total,
                hint: TRUNCATION_HINT.to_string(),
            }),
        })
    }
}

fn ok_response(data: CiJobStatusData) -> CiJobStatusResponse {
    CiJobStatusResponse {
        mutation_status: "ok",
        data,
        next_action_hint: None,
    }
}

fn estimate_envelope_overhead(run_id: u64, total: usize) -> usize {
    let probe = CiJobStatusData {
        run_id,
        jobs: Vec::new(),
        truncated: Some(CiJobTruncation {
            shown: 0,
            total,
            hint: TRUNCATION_HINT.to_string(),
        }),
    };
    serde_json::to_string(&probe)
        .map(|s| s.len())
        .unwrap_or(120)
}

pub fn compute_duration_ms(started_at: Option<&str>, completed_at: Option<&str>) -> Option<u64> {
    let start = chrono::DateTime::parse_from_rfc3339(started_at?).ok()?;
    let end = chrono::DateTime::parse_from_rfc3339(completed_at?).ok()?;
    let dur = end.signed_duration_since(start);
    let ms = dur.num_milliseconds();
    if ms >= 0 { Some(ms as u64) } else { None }
}

pub(in crate::bridge) async fn handle_ci_get_job_status(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id_raw = args.get("run_id").ok_or_else(|| {
        serialize_error(&CiJobStatusError::invalid_id("missing 'run_id' argument"))
    })?;
    let run_id = match run_id_raw.as_u64() {
        Some(v) if v > 0 => v,
        Some(_) => {
            return Err(serialize_error(&CiJobStatusError::invalid_id(
                "run_id must be a positive integer",
            )));
        }
        None => {
            return Err(serialize_error(&CiJobStatusError::invalid_id(
                "run_id must be an integer",
            )));
        }
    };
    let verbose = args
        .get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| serialize_error(&CiJobStatusError::auth_failed()))?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| serialize_error(&CiJobStatusError::auth_failed_with_detail(&e.to_string())))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/ci/jobs/{run_id}"));
    let mut url_for_verbose = url.clone();
    if verbose {
        url_for_verbose.push_str("?verbose=true");
    }
    let res = reqwest::Client::new()
        .get(&url_for_verbose)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| serialize_error(&CiJobStatusError::network_error(e.to_string())))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| serialize_error(&CiJobStatusError::network_error(e.to_string())))?;
    if status.is_success() {
        return Ok(body);
    }
    Err(body)
}

impl CiJobStatusError {
    pub fn auth_failed_with_detail(detail: &str) -> Self {
        Self {
            code: "auth_failed",
            message: format!("auth-token read failed: {detail}"),
            next_action: "Verify bpmux auth-token file is readable; restart bpmux to refresh"
                .to_string(),
        }
    }
}

fn serialize_error(err: &CiJobStatusError) -> String {
    serde_json::to_string(err).unwrap_or_else(|_| "{}".to_string())
}
