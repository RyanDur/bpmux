use super::types::serialize_error;
use crate::ci::CiJobStatusError;

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
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| serialize_error(&CiJobStatusError::auth_failed()))?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| serialize_error(&CiJobStatusError::auth_failed_with_detail(&e.to_string())))?;
    let url = local_url::api_url(port, &format!("/api/ci/jobs/{run_id}"));
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
