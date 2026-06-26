use super::helpers::urlencoding;

pub(in crate::bridge) fn curate_message(raw: &serde_json::Value) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "from": raw["from"].as_str()?,
        "text": raw["text"].as_str()?,
        "ts": raw["ts"].as_str().unwrap_or(""),
        "addressee": raw.get("addressee").and_then(|v| v.as_str()),
        "topic": raw.get("topic").and_then(|v| v.as_str()),
        "to": raw.get("to").and_then(|v| v.as_array()),
        "message_type": raw.get("message_type").and_then(|v| v.as_str()),
    }))
}

pub(in crate::bridge) async fn handle_read_messages(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    agent_name: &str,
) -> Result<String, String> {
    let default_limit = crate::transport::rest::DEFAULT_MESSAGE_LIMIT as u64;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default_limit);
    let short = agent_name.split(':').next().unwrap_or(agent_name);
    let url = crate::transport::local_url::api_url(port, &read_messages_request_path(limit, short));
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let msgs: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|_| body.clone())?;
    if msgs.is_empty() {
        return Ok("No messages.".into());
    }
    let curated: Vec<serde_json::Value> = msgs.iter().filter_map(curate_message).collect();
    serde_json::to_string_pretty(&curated).map_err(|e| format!("format: {e}"))
}

fn read_messages_request_path(limit: u64, agent_short: &str) -> String {
    format!(
        "/api/messages?limit={limit}&as={}",
        urlencoding(agent_short)
    )
}
