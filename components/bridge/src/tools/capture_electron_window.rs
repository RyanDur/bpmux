pub async fn handle_capture_electron_window(role: &str, port: u16) -> Result<String, String> {
    let token_path = auth::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = auth::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let role_encoded = url_encode(role);
    let url = local_url::api_url(port, &format!("/api/electron-capture?role={role_encoded}"));
    let res = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| {
            format!("capture_electron_window: electron-server unreachable at {url}: {e}")
        })?;
    let status = res.status();
    let body = res
        .text()
        .await
        .map_err(|e| format!("capture_electron_window: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("capture_electron_window: {status} {body}"));
    }
    Ok(body)
}

fn url_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                vec![c]
            } else {
                let mut buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut buf).as_bytes().to_vec();
                bytes
                    .iter()
                    .flat_map(|b| format!("%{b:02X}").chars().collect::<Vec<_>>())
                    .collect()
            }
        })
        .collect()
}
