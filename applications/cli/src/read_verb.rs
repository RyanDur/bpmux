pub fn run_read(limit: usize, full: bool) -> anyhow::Result<()> {
    let role = crate::bootstrap::resolve_role(&None);
    let port = crate::bootstrap::resolve_bpmux_port();
    let mut url = local_url::api_url(port, "/api/messages");
    url.push_str(&format!("?limit={limit}&chat_only=true&as={role}"));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let body = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        client.get(&url).send().await?.text().await
    })?;
    if full {
        println!("{body}");
        return Ok(());
    }
    let messages: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    for message in messages.iter().rev() {
        let sender = message["from"]
            .as_str()
            .unwrap_or("?")
            .split(':')
            .next()
            .unwrap_or("?");
        let text = message["text"].as_str().unwrap_or("");
        println!("{sender}: {text}");
    }
    Ok(())
}
