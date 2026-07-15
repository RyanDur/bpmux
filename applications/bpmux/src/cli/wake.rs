pub(crate) fn parse_wake_filter(
    filter: &Option<String>,
) -> anyhow::Result<Option<serde_json::Value>> {
    let Some(raw) = filter else {
        return Ok(None);
    };
    let parsed: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| anyhow::anyhow!("--filter must be a JSON object: {e}"))?;
    if !parsed.is_object() {
        anyhow::bail!("--filter must be a JSON object, e.g. '{{\"conclusion\":\"success\"}}'");
    }
    Ok(Some(parsed))
}

pub(crate) fn wake_payload(
    role: &str,
    user_id: &str,
    event: &str,
    filter: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert("role".to_string(), serde_json::json!(role));
    payload.insert("user_id".to_string(), serde_json::json!(user_id));
    payload.insert("event_class".to_string(), serde_json::json!(event));
    if let Some(parsed) = filter {
        payload.insert("filter".to_string(), parsed);
    }
    serde_json::Value::Object(payload)
}

pub(crate) fn run_wake_on_event(event: &str, filter: &Option<String>) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let filter_value = parse_wake_filter(filter)?;
    let cwd = std::env::current_dir()?;
    let user_id = crate::resolve_registered_user_id(&cwd).ok_or_else(|| {
        anyhow::anyhow!(
            "no registered user-id for this dir — run `bpmux register` first: a commitment without an id can never fire"
        )
    })?;
    let body = wake_payload(&role, &user_id, event, filter_value);
    let port = crate::resolve_bpmux_port();
    let url = local_url::api_url(port, "/api/wake-on-event");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, reason) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let response = client.post(&url).json(&body).send().await?;
        let status = response.status();
        let reason = response.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, reason))
    })?;
    if !status.is_success() {
        anyhow::bail!("wake-on-event rejected ({status}): {reason}");
    }
    let scope = match filter {
        Some(raw) => format!("{event} {raw}"),
        None => event.to_string(),
    };
    println!("✓ committed {role} → wake on {scope}");
    Ok(())
}
