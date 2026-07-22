pub async fn handle_get_sync_status(port: u16) -> Result<String, String> {
    let token_path = auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = local_url::api_url(port, "/api/sync-status");
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("get_sync_status: electron-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("get_sync_status: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("get_sync_status: {status} — {body}"));
    }
    Ok(body)
}
