pub(crate) fn http_get(url: &str) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result: Result<(reqwest::StatusCode, String), reqwest::Error> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        Ok((status, body))
    });
    Ok(result?)
}

fn bearer_token() -> anyhow::Result<String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    crate::session::auth_token::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))
}

pub(crate) fn authed_get(path: &str) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let port = crate::resolve_bpmux_port();
    let token = bearer_token()?;
    let url = crate::transport::local_url::api_url(port, path);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result: Result<(reqwest::StatusCode, String), reqwest::Error> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let resp = client
            .get(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        Ok((status, body))
    });
    Ok(result?)
}

pub(crate) fn authed_post_empty(path: &str) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let port = crate::resolve_bpmux_port();
    let token = bearer_token()?;
    let url = crate::transport::local_url::api_url(port, path);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result: Result<(reqwest::StatusCode, String), reqwest::Error> = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(10_000))
            .build()?;
        let resp = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        Ok((status, body))
    });
    Ok(result?)
}
