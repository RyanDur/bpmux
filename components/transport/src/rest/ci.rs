use super::shared::{TrackerHandle, err_body, json_headers};
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn get_ci(
    axum::Extension(select): axum::Extension<store::Select>,
    axum::Extension(actions_url): axum::Extension<ci::ActionsUrl>,
) -> impl IntoResponse {
    let report = select.select(|state| ci::slice::report_from_state(&state.ci, &actions_url));
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string()),
    )
}

pub async fn get_ci_log(
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    axum::Extension(ci_log_store): axum::Extension<ci::CiLogStore>,
    axum::extract::Path(run_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
    if let Some(cached) = ci_log_store.read(run_id) {
        return (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&cached).unwrap_or_else(|_| "{}".to_string()),
        );
    }
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_failed_tests_for_run(run_id).await {
        Ok(failures) => {
            ci_log_store.write(run_id, failures.clone());
            (
                StatusCode::OK,
                json_headers(),
                serde_json::to_string(&failures).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(err) => {
            let status = match &err {
                ci::CiLogError::RunNotFound { .. } => StatusCode::NOT_FOUND,
                ci::CiLogError::RunInProgress { .. } => StatusCode::ACCEPTED,
                ci::CiLogError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
                ci::CiLogError::UpstreamError { .. } => StatusCode::BAD_GATEWAY,
            };
            (
                status,
                json_headers(),
                serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()),
            )
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct CiJobsQuery {
    #[serde(default)]
    pub verbose: bool,
}

pub async fn get_ci_jobs(
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    axum::extract::Path(run_id): axum::extract::Path<u64>,
    axum::extract::Query(query): axum::extract::Query<CiJobsQuery>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.fetch_job_status_for_run(run_id).await {
        Ok(jobs) => {
            let response = bridge::tools::build_ci_job_status_response(run_id, jobs, query.verbose);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(err) => {
            let status = match err.code {
                "not_found" => StatusCode::NOT_FOUND,
                "auth_failed" => StatusCode::UNAUTHORIZED,
                "rate_limit" => StatusCode::TOO_MANY_REQUESTS,
                "quota_exhausted" => StatusCode::PAYMENT_REQUIRED,
                "invalid_id" => StatusCode::BAD_REQUEST,
                _ => StatusCode::BAD_GATEWAY,
            };
            (
                status,
                json_headers(),
                serde_json::to_string(&err).unwrap_or_else(|_| "{}".to_string()),
            )
        }
    }
}

pub async fn post_ci_rerun(
    axum::Extension(tracker_cell): axum::Extension<super::shared::TrackerCell>,
    axum::extract::Path(run_id): axum::extract::Path<u64>,
) -> impl IntoResponse {
    let tracker: TrackerHandle = tracker_cell.current();
    let Some(tracker) = tracker else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("tracker not configured"),
        );
    };
    match tracker.rerun_failed_jobs(run_id).await {
        Ok(()) => (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&serde_json::json!({
                "status": "success",
                "run_id": run_id,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(err) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            serde_json::to_string(&serde_json::json!({
                "status": "failure",
                "run_id": run_id,
                "message": err,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
        ),
    }
}
