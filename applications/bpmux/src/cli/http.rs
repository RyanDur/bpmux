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

pub(crate) fn http_get_with_query(
    path: &str,
    params: &[(&str, &str)],
) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let port = crate::resolve_bpmux_port();
    let mut url = local_url::api_url(port, path);
    for (i, (key, value)) in params.iter().enumerate() {
        let separator = if i == 0 { '?' } else { '&' };
        url.push(separator);
        url.push_str(key);
        url.push('=');
        url.push_str(&local_url::percent_encode(value));
    }
    http_get(&url)
}

fn bearer_token() -> anyhow::Result<String> {
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    crate::auth::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))
}

pub(crate) fn authed_get(path: &str) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let port = crate::resolve_bpmux_port();
    let token = bearer_token()?;
    let url = local_url::api_url(port, path);
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
    let url = local_url::api_url(port, path);
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
