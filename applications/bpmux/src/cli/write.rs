pub(crate) fn post_rpc_write(
    tool: &str,
    agent: &str,
    args: &serde_json::Value,
) -> anyhow::Result<String> {
    let port = crate::resolve_bpmux_port();
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve auth-token path (HOME unset?)"))?;
    let token = crate::session::auth_token::read(&token_path)
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

pub(crate) fn run_pass(number: u64, acceptor: &str) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(number));
    args.insert("acceptor".to_string(), serde_json::json!(acceptor));
    args.insert("would_file_bug".to_string(), serde_json::json!(false));
    let message = post_rpc_write("pass_desk_check", &role, &serde_json::Value::Object(args))?;
    println!("{message}");
    Ok(())
}
