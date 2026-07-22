const WAIT_CLIENT_GRACE_MS: u64 = 5_000;

pub fn emit_recall_activity(role: &str, count: usize) {
    let body = memory::recall::build_recall_activity_payload(role, count);
    post_activity(&body);
}

pub fn run_wait_for_event(after: &Option<String>, timeout_ms: u64) -> anyhow::Result<()> {
    let role = crate::bootstrap::resolve_role(&None);
    let port = crate::bootstrap::resolve_bpmux_port();
    let mut url = local_url::api_url(port, "/api/wait-for-event");
    url.push_str(&format!("?as={role}&timeout_ms={timeout_ms}"));
    if let Some(cursor) = after {
        url.push_str(&format!("&after={cursor}"));
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let body = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                timeout_ms + WAIT_CLIENT_GRACE_MS,
            ))
            .build()?;
        client.get(&url).send().await?.text().await
    })?;
    println!("{body}");
    Ok(())
}

pub fn send_payload(
    role: &str,
    text: &str,
    to: &[String],
    addressee: &Option<String>,
    sender_id: Option<String>,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "from".to_string(),
        serde_json::json!(format!("{role}:claude-code")),
    );
    payload.insert("text".to_string(), serde_json::json!(text));
    if let Some(id) = sender_id {
        payload.insert("sender_id".to_string(), serde_json::json!(id));
    }
    if !to.is_empty() {
        payload.insert("to".to_string(), serde_json::json!(to));
    }
    if let Some(target) = addressee {
        payload.insert("addressee".to_string(), serde_json::json!(target));
    }
    serde_json::Value::Object(payload)
}

pub fn run_send(text: &str, to: &[String], addressee: &Option<String>) -> anyhow::Result<()> {
    let role = crate::bootstrap::resolve_role(&None);
    let sender_id = std::env::current_dir()
        .ok()
        .and_then(|cwd| crate::bootstrap::resolve_registered_user_id(&cwd));
    let body = send_payload(&role, text, to, addressee, sender_id);
    let port = crate::bootstrap::resolve_bpmux_port();
    let url = local_url::api_url(port, "/api/messages");
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
        anyhow::bail!("send rejected ({status}): {reason}");
    }
    let destination = if to.is_empty() {
        "broadcast".to_string()
    } else {
        format!("[{}]", to.join(", "))
    };
    println!("✓ sent → {destination}");
    Ok(())
}

pub fn run_register() -> anyhow::Result<()> {
    let role = crate::bootstrap::resolve_role(&None);
    let cwd = std::env::current_dir()?;
    let pinned = std::env::var("BPMUX_USER_ID")
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false);
    let user_id = crate::bootstrap::resolve_or_register_user_id(&cwd)?;
    let mut payload = serde_json::Map::new();
    payload.insert("user_id".to_string(), serde_json::json!(user_id));
    payload.insert("display".to_string(), serde_json::json!(role));
    payload.insert("role".to_string(), serde_json::json!(role));
    if let Some(terminal_id) = std::env::var("BPMUX_TERMINAL_ID")
        .ok()
        .filter(|id| !id.trim().is_empty())
    {
        payload.insert("terminal_id".to_string(), serde_json::json!(terminal_id));
    }
    let body = serde_json::Value::Object(payload);
    let port = crate::bootstrap::resolve_bpmux_port();
    let url = local_url::api_url(port, "/api/register");
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
        anyhow::bail!("register rejected ({status}): {reason}");
    }
    println!("✓ registered {role} ({user_id})");
    if !pinned {
        println!("  pin this identity so it survives a cd: export BPMUX_USER_ID={user_id}");
    }
    Ok(())
}

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

pub fn activity_payload(from: &str, text: &str) -> serde_json::Value {
    serde_json::json!({
        "from": from,
        "text": text,
        "message_type": "activity",
    })
}

pub fn run_emit_activity(from: &str, text: &str) -> anyhow::Result<()> {
    post_activity(&activity_payload(from, text));
    Ok(())
}

fn post_activity(body: &serde_json::Value) {
    let port = crate::bootstrap::resolve_bpmux_port();
    let url = local_url::api_url(port, "/api/messages");
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build();
        let Ok(client) = client else { return };
        let _ = client.post(&url).json(&body).send().await;
    });
}
