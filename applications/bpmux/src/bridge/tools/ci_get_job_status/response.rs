use super::types::{CiJobStatusData, CiJobStatusResponse, CiJobTruncation};
use crate::ci::CiJobRecord;

pub const MAX_DEFAULT_BUDGET_BYTES: usize = 2048;
pub(super) const TRUNCATION_HINT: &str = "set verbose=true for full payload";

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
