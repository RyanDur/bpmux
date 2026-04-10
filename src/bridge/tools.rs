//! Bridge tool handlers — one function per MCP tool.

use futures::SinkExt;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use crate::transport::protocol::ClientMessage;

type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

pub(super) async fn ws_send(ws_tx: &Mutex<Option<WsSink>>, payload: String) -> Result<(), String> {
    let mut tx = ws_tx.lock().await;
    let tx = tx
        .as_mut()
        .ok_or("not connected to bpmux — reconnecting, try again in a few seconds")?;
    tx.send(tungstenite::Message::Text(payload.into()))
        .await
        .map_err(|e| format!("send failed: {e}"))
}

pub(super) async fn handle_send(
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
    last_channel: &Mutex<Option<Vec<String>>>,
) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("missing 'text' argument")?;
    let explicit_to = args.get("to").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>()
    });
    let to = match explicit_to {
        Some(t) => Some(t),
        None => last_channel.lock().await.clone(),
    };
    // Attention signal (#158) — optional `addressee` arg identifies the
    // role whose turn it is to respond. Agents without that role see the
    // message but should skip responding. See docs/turn-aware-channel-spec.md.
    let addressee = args
        .get("addressee")
        .and_then(|v| v.as_str())
        .map(String::from);
    let message_type = args
        .get("message_type")
        .and_then(|v| v.as_str())
        .map(String::from);
    let json = serde_json::to_string(&ClientMessage::Send {
        text: text.to_string(),
        to,
        addressee,
        message_type,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok("message sent".into())
}

pub(super) async fn handle_set_name(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &Mutex<String>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let new_name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument")?
        .to_string();
    *name.lock().await = new_name.clone();
    let json = serde_json::json!({"type": "rename", "name": new_name}).to_string();
    ws_send(ws_tx, json).await?;
    Ok(format!("name set to {new_name}"))
}

pub(super) async fn handle_set_status(
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or("missing 'status' argument")?;
    let json = serde_json::to_string(&ClientMessage::Send {
        text: status.to_string(),
        to: None,
        addressee: None,
        message_type: Some("activity".into()),
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok(format!("status set to {status}"))
}

pub(super) async fn handle_read_messages(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let default_limit = crate::transport::rest::DEFAULT_MESSAGE_LIMIT as u64;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default_limit);
    let url = format!("http://127.0.0.1:{port}/api/messages?limit={limit}");
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
    Ok(msgs
        .iter()
        .filter_map(|m| {
            let from = m["from"].as_str()?;
            let text = m["text"].as_str()?;
            Some(format!("[{from}] {text}"))
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(super) async fn handle_search_log(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("missing 'query' argument")?;
    let mut url = format!(
        "http://127.0.0.1:{port}/api/search?query={}",
        urlencoding(query)
    );
    if let Some(sender) = args.get("sender").and_then(|v| v.as_str()) {
        url.push_str(&format!("&sender={sender}"));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to search: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let results: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|_| body.clone())?;
    if results.is_empty() {
        return Ok(format!("No results for \"{query}\"."));
    }
    let mut text = format!("── Search: \"{query}\" ({} results) ──\n", results.len());
    for (i, result) in results.iter().enumerate() {
        text.push_str(&format!("\n[{}]\n", i + 1));
        if let Some(msgs) = result["messages"].as_array() {
            for msg in msgs {
                let from = msg["from"].as_str().unwrap_or("?");
                let sender = from.split(':').next().unwrap_or(from);
                let msg_text = msg["text"].as_str().unwrap_or("");
                text.push_str(&format!("  {sender}: {msg_text}\n"));
            }
        }
    }
    text.push_str("── End of results ──");
    Ok(text)
}

pub(super) fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '?' => "%3F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

pub(super) async fn handle_save_lesson(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    name: &Mutex<String>,
) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .ok_or("missing 'topic' argument")?;
    let lesson = args
        .get("lesson")
        .and_then(|v| v.as_str())
        .ok_or("missing 'lesson' argument")?;
    let evidence: Vec<String> = args
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let role = name.lock().await.clone();
    let body = serde_json::json!({
        "topic": topic,
        "lesson": lesson,
        "author_role": role,
        "evidence": evidence,
    });
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/lessons");
    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to save: {e}"))?;
    Ok(format!("lesson saved: {topic}"))
}

pub(super) async fn handle_list_lessons(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let mut url = format!("http://127.0.0.1:{port}/api/lessons");
    let mut has_param = false;
    if let Some(topic) = args.get("topic").and_then(|v| v.as_str()) {
        url.push_str(&format!("?topic={}", urlencoding(topic)));
        has_param = true;
    }
    if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
        url.push_str(if has_param { "&" } else { "?" });
        url.push_str(&format!("limit={limit}"));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read: {e}"))?;
    Ok(body)
}

pub(super) async fn handle_list_peer_lessons(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let mut url = format!("http://127.0.0.1:{port}/api/peer-lessons");
    if let Some(peer) = args.get("peer").and_then(|v| v.as_str()) {
        url.push_str(&format!("?peer={}", urlencoding(peer)));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read: {e}"))?;
    Ok(body)
}

pub(super) async fn handle_simulate_click(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let x = args
        .get("x")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'x'")? as u16;
    let y = args
        .get("y")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'y'")? as u16;
    let body = serde_json::json!({"x": x, "y": y});
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/simulate/click");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed: {e}"))?;
    resp.text().await.map_err(|e| format!("failed: {e}"))
}

pub(super) async fn handle_simulate_key(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("missing 'key'")?;
    let body = serde_json::json!({"key": key});
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/simulate/key");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed: {e}"))?;
    resp.text().await.map_err(|e| format!("failed: {e}"))
}

pub(super) async fn handle_capture_screen(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/api/screen");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to capture: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read: {e}"))?;
    Ok(body)
}
