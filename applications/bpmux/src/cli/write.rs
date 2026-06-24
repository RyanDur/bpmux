pub(crate) fn post_rpc_write(
    tool: &str,
    agent: &str,
    args: &serde_json::Value,
) -> anyhow::Result<String> {
    let port = crate::resolve_bpmux_port();
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, &format!("/api/rpc/{tool}"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, text) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(crate::transport::rpc_routes::AGENT_HEADER, agent)
            .json(args)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, text))
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if status.is_success() {
        parsed
            .get("message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "bpmux {tool}: success {status} but response missing 'message': {text}"
                )
            })
    } else {
        let reason = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or(text.as_str())
            .to_string();
        anyhow::bail!("bpmux {tool} rejected ({status}): {reason}")
    }
}

pub(super) fn dispatch_write(
    tool: &str,
    args: serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let message = post_rpc_write(tool, &role, &serde_json::Value::Object(args))?;
    println!("{message}");
    Ok(())
}

pub(crate) fn run_signal(state: &str) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let port = crate::resolve_bpmux_port();
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    let token = crate::auth::read(&token_path)
        .map_err(|e| anyhow::anyhow!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, "/api/presence");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, text) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .header(crate::transport::rpc_routes::AGENT_HEADER, role.as_str())
            .json(&serde_json::json!({ "state": state }))
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, text))
    })?;
    if status.is_success() {
        println!("{text}");
        Ok(())
    } else {
        anyhow::bail!("bpmux signal rejected ({status}): {text}")
    }
}
