use super::helpers::urlencoding;

/// Curate a raw message JSON value into the shape agents consume via
/// read_messages. Returns None if required fields (`from`, `text`) are
/// missing or malformed — those messages are skipped entirely.
///
/// Optional fields (addressee, topic, to, message_type) are preserved
/// as null when absent so agents can distinguish "broadcast" (null) from
/// "addressed to someone else" (a role name). This is the REST/JSON
/// convention; the channel-notification path in bridge/mod.rs uses
/// omit-when-absent because Claude Code v2.1.109+ rejects null meta.
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
    // Pass the agent's short name as `?as=` so the REST endpoint
    // applies the topic-subscription filter — matching the WebSocket
    // push path. Without this the agent reads messages it would not
    // see via notifications, silently diverging the two paths.
    let short = agent_name.split(':').next().unwrap_or(agent_name);
    let as_encoded = urlencoding(short);
    let url = format!("http://127.0.0.1:{port}/api/messages?limit={limit}&as={as_encoded}");
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
