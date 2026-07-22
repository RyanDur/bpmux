pub async fn handle_get_ci_log_for_run(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'run_id' argument")?;
    let token_path = auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = local_url::api_url(port, &format!("/api/ci/log/{run_id}"));
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
