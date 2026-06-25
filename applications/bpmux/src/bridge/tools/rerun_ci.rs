pub(in crate::bridge) async fn handle_rerun_ci(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'run_id' argument (u64 expected)")?;
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/ci/rerun/{run_id}"));
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("rerun_ci: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("rerun_ci: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("rerun_ci: {status} — {body}"));
    }
    Ok(body)
}
