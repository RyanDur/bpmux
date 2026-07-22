#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::ci_get_job_status::{MAX_DEFAULT_BUDGET_BYTES, build_ci_job_status_response};
use ci::{CiJobRecord, CiJobStatusError, compute_duration_ms};

fn job(name: &str, conclusion: &str, ms: Option<u64>, html: &str) -> CiJobRecord {
    CiJobRecord {
        name: name.to_string(),
        conclusion: conclusion.to_string(),
        duration_ms: ms,
        started_at: Some("2026-05-20T03:00:00Z".to_string()),
        completed_at: Some("2026-05-20T03:02:00Z".to_string()),
        html_url: html.to_string(),
    }
}

#[test]
fn three_job_workflow_returns_per_job_records_with_all_fields() {
    let jobs = vec![
        job(
            "test",
            "failure",
            Some(120_000),
            "https://github.com/acme/bpmux/actions/runs/1/job/100",
        ),
        job(
            "build-bpmux",
            "success",
            Some(90_000),
            "https://github.com/acme/bpmux/actions/runs/1/job/101",
        ),
        job(
            "playwright-e2e",
            "success",
            Some(300_000),
            "https://github.com/acme/bpmux/actions/runs/1/job/102",
        ),
    ];
    let response = build_ci_job_status_response(1, jobs, false);
    assert_eq!(response.mutation_status, "ok");
    assert_eq!(response.data.run_id, 1);
    assert_eq!(response.data.jobs.len(), 3);
    assert!(response.data.truncated.is_none());
    let failed = response
        .data
        .jobs
        .iter()
        .find(|j| j.name == "test")
        .expect("failed job present");
    assert_eq!(failed.conclusion, "failure");
    assert_eq!(failed.duration_ms, Some(120_000));
    assert!(failed.started_at.is_some());
    assert!(failed.completed_at.is_some());
    assert!(failed.html_url.starts_with("https://"));
    let serialized = serde_json::to_value(&response).unwrap();
    assert_eq!(serialized["mutation_status"], "ok");
    let job_array = serialized["data"]["jobs"].as_array().unwrap();
    assert_eq!(job_array.len(), 3);
    let payload = &job_array[0];
    for field in [
        "name",
        "conclusion",
        "duration_ms",
        "started_at",
        "completed_at",
        "html_url",
    ] {
        assert!(
            !payload[field].is_null(),
            "per-job record must carry field {field}"
        );
    }
}

#[test]
fn invalid_run_id_returns_canonical_error_shape() {
    let err = CiJobStatusError::not_found(99_999_999);
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["code"], "not_found");
    assert!(json["message"].as_str().unwrap().contains("99999999"));
    let next_action = json["next_action"].as_str().unwrap();
    assert!(
        !next_action.is_empty(),
        "next_action must guide the agent to a recovery path"
    );
    assert!(
        next_action.to_lowercase().contains("get_ci_state")
            || next_action.to_lowercase().contains("run_id"),
        "next_action must mention discovery mechanism: {next_action}"
    );
    for class in [
        CiJobStatusError::invalid_id("missing run_id"),
        CiJobStatusError::quota_exhausted(),
        CiJobStatusError::network_error("timed out"),
        CiJobStatusError::auth_failed(),
        CiJobStatusError::rate_limit(60),
    ] {
        let j = serde_json::to_value(&class).unwrap();
        assert!(j["code"].is_string());
        assert!(j["message"].is_string());
        assert!(j["next_action"].is_string());
    }
}

#[test]
fn verbose_flag_bypasses_token_economy_budget() {
    let many_jobs: Vec<CiJobRecord> = (0..20)
        .map(|i| {
            job(
                &format!("playwright-shard-{i}-of-20"),
                "success",
                Some(180_000),
                &format!("https://github.com/acme/bpmux/actions/runs/9999999999/job/123456789{i}"),
            )
        })
        .collect();
    let default_response = build_ci_job_status_response(9_999_999_999, many_jobs.clone(), false);
    let default_size = serde_json::to_string(&default_response).unwrap().len();
    assert!(
        default_size <= MAX_DEFAULT_BUDGET_BYTES + 64,
        "default response must respect ≤2KB budget; got {default_size} bytes"
    );
    assert!(
        default_response.data.truncated.is_some(),
        "default mode must emit truncation marker when payload would exceed budget"
    );
    let truncated = default_response.data.truncated.as_ref().unwrap();
    assert_eq!(truncated.total, 20);
    assert!(truncated.shown < 20);
    assert!(truncated.hint.contains("verbose"));
    let verbose_response = build_ci_job_status_response(9_999_999_999, many_jobs, true);
    let verbose_size = serde_json::to_string(&verbose_response).unwrap().len();
    assert_eq!(verbose_response.data.jobs.len(), 20);
    assert!(verbose_response.data.truncated.is_none());
    assert!(
        verbose_size > MAX_DEFAULT_BUDGET_BYTES,
        "verbose payload should exceed default budget (proving bypass): {verbose_size} bytes"
    );
}

#[test]
fn compute_duration_ms_handles_iso8601_timestamps() {
    let dur = compute_duration_ms(Some("2026-05-20T03:00:00Z"), Some("2026-05-20T03:02:30Z"));
    assert_eq!(dur, Some(150_000));
}

#[test]
fn compute_duration_ms_returns_none_for_missing_timestamps() {
    assert_eq!(
        compute_duration_ms(None, Some("2026-05-20T03:02:30Z")),
        None
    );
    assert_eq!(
        compute_duration_ms(Some("2026-05-20T03:00:00Z"), None),
        None
    );
    assert_eq!(compute_duration_ms(None, None), None);
}

#[test]
fn small_workflow_under_budget_not_truncated_in_default_mode() {
    let jobs = vec![
        job(
            "test",
            "success",
            Some(60_000),
            "https://github.com/x/y/actions/runs/1/job/100",
        ),
        job(
            "build",
            "success",
            Some(60_000),
            "https://github.com/x/y/actions/runs/1/job/101",
        ),
    ];
    let response = build_ci_job_status_response(1, jobs, false);
    assert!(response.data.truncated.is_none());
    assert_eq!(response.data.jobs.len(), 2);
}
