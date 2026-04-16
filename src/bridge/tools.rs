//! Bridge tool handlers — one function per MCP tool.

use futures::SinkExt;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use crate::transport::protocol::ClientMessage;

pub(super) type WsSink = futures::stream::SplitSink<
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
    let topic = args.get("topic").and_then(|v| v.as_str()).map(String::from);
    let json = serde_json::to_string(&ClientMessage::Send {
        text: text.to_string(),
        to,
        addressee,
        message_type,
        topic,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok("message sent".into())
}

/// Parse the `scope` arg into a target-recipient list. "all" (the
/// BROADCAST_ALL sentinel) widens to everyone and returns None;
/// other strings are split on comma into explicit recipients.
/// Returns Err when the arg is empty or only whitespace.
pub(super) fn parse_share_scope(scope: &str) -> Result<Option<Vec<String>>, String> {
    if scope.is_empty() {
        return Err("empty scope".to_string());
    }
    if scope == crate::session::message::BROADCAST_ALL {
        return Ok(None);
    }
    let recipients: Vec<String> = scope
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if recipients.is_empty() {
        return Err("empty scope".to_string());
    }
    Ok(Some(recipients))
}

/// Format the provenance marker an agent's share carries into the
/// target scope. Form 1 (re-share) includes source-scope; Form 2
/// (fresh share) omits it because there's no source thread.
pub(super) fn format_share_marker(agent: &str, source_scope: Option<&str>, text: &str) -> String {
    match source_scope {
        Some(s) => format!("[Shared by {agent} from {s} thread] {text}"),
        None => format!("[Shared by {agent}] {text}"),
    }
}

/// Pick the agent's most recent visible chat message for Form 1
/// re-share. `messages` is the list the REST `/api/messages?as=<agent>`
/// endpoint returns — most-recent-first. Returns the source message,
/// or None when the agent has no shareable recent chat in view.
/// Skips non-chat message types (activity, system, etc.).
pub(super) fn pick_most_recent_visible_chat(
    messages: &[serde_json::Value],
) -> Option<&serde_json::Value> {
    messages.iter().find(|m| {
        // Chat is represented by missing or null message_type in
        // the REST output (the default variant). Any explicit
        // message_type string is a non-chat kind we should skip.
        m.get("message_type").map(|v| v.is_null()).unwrap_or(true)
    })
}

/// `share_message` tool — widen a recent message (Form 1) or send
/// fresh text (Form 2) to a wider scope with provenance marking.
/// Mirrors the TUI's `/share` semantic for agents, including the
/// explicit-privacy refusal: messages the source sender marked
/// private cannot be re-shared (agents must paraphrase in their
/// own words if they need wider visibility).
pub(super) async fn handle_share_message(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let scope_str = args
        .get("scope")
        .and_then(|v| v.as_str())
        .ok_or("missing 'scope' argument (use \"all\" or a comma-list of roles)")?;
    let target_to =
        parse_share_scope(scope_str).map_err(|_| format!("invalid scope: {scope_str:?}"))?;
    let fresh_text = args
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let short = agent_name.split(':').next().unwrap_or(agent_name);

    // Form 2: explicit new text.
    if let Some(body) = fresh_text {
        let marker = format_share_marker(short, None, body);
        let json = serde_json::to_string(&ClientMessage::Send {
            text: marker,
            to: target_to.clone(),
            addressee: None,
            message_type: None,
            topic: None,
        })
        .map_err(|e| format!("failed to serialize: {e}"))?;
        ws_send(ws_tx, json).await?;
        return Ok(format!(
            "shared fresh message to {}",
            scope_str_or_all(scope_str)
        ));
    }

    // Form 1: propagate the agent's most recent visible chat message.
    // Fetch the agent's visibility-scoped recent log via the REST
    // surface — same filter the WS push path uses, so what the tool
    // sees matches what the agent saw.
    let as_encoded = urlencoding(short);
    let url = format!("http://127.0.0.1:{port}/api/messages?limit=20&as={as_encoded}");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch log: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read log: {e}"))?;
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse log: {e}"))?;
    let Some(source) = pick_most_recent_visible_chat(&messages) else {
        return Err("No recent visible chat to share.".to_string());
    };

    // Privacy gate — refuse to widen explicit-privacy messages.
    if source
        .get("explicit_privacy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err(
            "cannot share a private message; paraphrase it in your own words if it needs wider visibility"
                .to_string(),
        );
    }

    let source_text = source
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("source message has no text")?;
    let source_to = source.get("to").and_then(|v| v.as_array());
    let source_scope = match source_to {
        None => "broadcast".to_string(),
        Some(recipients) if recipients.is_empty() => "broadcast".to_string(),
        Some(_) => {
            // For a DM between agent and one counterpart, report the
            // counterpart as the source scope — matches the TUI's
            // channel_name convention.
            let sender_short = source
                .get("from")
                .and_then(|v| v.as_str())
                .and_then(|s| s.split(':').next())
                .unwrap_or("peer");
            sender_short.to_string()
        }
    };
    let marker = format_share_marker(short, Some(&source_scope), source_text);

    // addressee is explicitly NOT inherited. Propagated messages are
    // awareness updates, not tasks for the original recipient.
    let json = serde_json::to_string(&ClientMessage::Send {
        text: marker,
        to: target_to,
        addressee: None,
        message_type: None,
        topic: None,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok(format!(
        "shared recent message to {}",
        scope_str_or_all(scope_str)
    ))
}

fn scope_str_or_all(scope: &str) -> &str {
    if scope == crate::session::message::BROADCAST_ALL {
        "all"
    } else {
        scope
    }
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
        topic: None,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok(format!("status set to {status}"))
}

/// Curate a raw message JSON value into the shape agents consume via
/// read_messages. Returns None if required fields (`from`, `text`) are
/// missing or malformed — those messages are skipped entirely.
///
/// Optional fields (addressee, topic, to, message_type) are preserved
/// as null when absent so agents can distinguish "broadcast" (null) from
/// "addressed to someone else" (a role name). This is the REST/JSON
/// convention; the channel-notification path in bridge/mod.rs uses
/// omit-when-absent because Claude Code v2.1.109+ rejects null meta.
pub(super) fn curate_message(raw: &serde_json::Value) -> Option<serde_json::Value> {
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

pub(super) async fn handle_read_messages(
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
        url.push_str(&format!("&sender={}", urlencoding(sender)));
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
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{b:02X}"),
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
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to save: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("save failed ({status}): {body}"));
    }
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
    let token = load_auth_token()?;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/simulate/click");
    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
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
    // Forward `modifiers` if the caller supplied it (#170). Accept an
    // array of strings; let the REST endpoint validate names so the
    // bridge doesn't need to know the modifier vocabulary.
    let mut body = serde_json::json!({"key": key});
    if let Some(modifiers) = args.get("modifiers") {
        body["modifiers"] = modifiers.clone();
    }
    let token = load_auth_token()?;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/simulate/key");
    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed: {e}"))?;
    resp.text().await.map_err(|e| format!("failed: {e}"))
}

/// Read the per-run auth token that the server wrote at startup. Two
/// distinct failure strings so an agent invoking a simulate tool knows
/// whether bpmux hasn't started yet vs. the file is there but we can't
/// read it.
fn load_auth_token() -> Result<String, String> {
    let path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "auth token path unavailable (no home directory)".to_string())?;
    match crate::session::auth_token::read(&path) {
        Ok(t) if !t.is_empty() => Ok(t),
        Ok(_) => Err(format!(
            "Auth token not found at {}. Start bpmux first to generate it.",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(format!(
            "Auth token not found at {}. Start bpmux first to generate it.",
            path.display()
        )),
        Err(e) => Err(format!(
            "Auth token unreadable at {}. Check permissions. ({e})",
            path.display()
        )),
    }
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

/// Post-compaction recovery — bookmark survives context resets.
pub(super) async fn handle_save_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use crate::session::bookmark::{Bookmark, save_bookmark};
    let session_dir = get_session_dir().await?;
    let bookmark = Bookmark {
        current_task: args
            .get("current_task")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        decisions: args
            .get("decisions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        open_questions: args
            .get("open_questions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        next_step: args
            .get("next_step")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        notes: args
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
    };
    let role = name.split(':').next().unwrap_or(name);
    save_bookmark(&session_dir, role, &bookmark).map_err(|e| format!("save failed: {e}"))?;
    Ok("bookmark saved".to_string())
}

/// Bookmark recovery companion to save_bookmark.
pub(super) async fn handle_get_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use crate::session::bookmark::load_bookmark;
    let session_dir = get_session_dir().await?;
    let default_role = name.split(':').next().unwrap_or(name);
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let bookmark = load_bookmark(&session_dir, role);
    serde_json::to_string_pretty(&bookmark).map_err(|e| format!("serialize: {e}"))
}

async fn get_session_dir() -> Result<std::path::PathBuf, String> {
    use crate::session::log::default_sessions_base;
    let base = default_sessions_base().map_err(|e| format!("sessions base: {e}"))?;
    let current = base.join("current");
    if current.is_symlink() {
        std::fs::read_link(&current).map_err(|e| format!("read current: {e}"))
    } else {
        Err("no current session".to_string())
    }
}

/// Per-role identity traits loaded at session start.
pub(super) async fn handle_get_identity(
    args: &serde_json::Map<String, serde_json::Value>,
    default_role: &str,
) -> Result<String, String> {
    use crate::session::identity::{default_identity_dir, load_identity};
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let base = default_identity_dir().map_err(|e| format!("identity dir: {e}"))?;
    let index = load_identity(&base, role);
    serde_json::to_string_pretty(&index).map_err(|e| format!("serialize: {e}"))
}

/// `get_token_report` (#179) — reads the token log directly from disk
/// and returns a structured summary as JSON. Bypasses the HTTP API
/// because the log is a file on the local filesystem that both the
/// bpmux TUI process and the bridge process can read independently.
/// Returns an empty report (no rows, zero totals) if the log file
/// doesn't exist yet — the first-use case, not an error.
pub(super) async fn handle_get_token_report() -> Result<String, String> {
    use crate::session::tokens::{build_token_report, default_tokens_log_path, read_token_rows};
    let path = default_tokens_log_path().map_err(|e| format!("path resolution failed: {e}"))?;
    let rows = read_token_rows(&path).map_err(|e| format!("log read failed: {e}"))?;
    let report = build_token_report(&rows);
    serde_json::to_string_pretty(&report).map_err(|e| format!("serialization failed: {e}"))
}

/// `view_velocity` (#209a) — fetch the recent message log, parse
/// workflow-signal timestamps, and return velocity metrics as JSON.
/// Pure computation over the log; every number traces back to a
/// signal that actually fired (see `session::velocity::compute`).
pub(super) async fn handle_view_velocity(port: u16) -> Result<String, String> {
    // Fetch a wide window so the 14-day compute has enough rows; no
    // `?as=` because workflow signals are broadcasts, not topic-
    // scoped. Limit is generous but bounded to avoid unbounded reads.
    let url = format!("http://127.0.0.1:{port}/api/messages?limit=2000");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let messages: Vec<crate::session::Message> =
        serde_json::from_str(&body).map_err(|e| format!("parse messages: {e}: {body}"))?;
    let metrics = crate::session::velocity::compute(&messages, chrono::Utc::now());
    serde_json::to_string_pretty(&metrics).map_err(|e| format!("serialize: {e}"))
}

/// Classifier misfire feedback handler (#159).
///
/// Agents call this tool when they observe a classifier nudge firing on a
/// message that isn't actually a learning moment. POSTs to bpmux's
/// `/api/classifier-misfire` which writes a `MISFIRE` line to the diag
/// log (gated on `BPMUX_LOG` being set). No effect on the classifier
/// itself — this is purely a training-signal capture path so agent
/// knowledge about wrong fires gets externalized instead of silently
/// dismissed.
///
/// The `reason` argument is OPTIONAL per the acceptance criteria: agents
/// can mark a fire as wrong without having to explain why. Per Naur's
/// "state observation, not conclusion" framing.
pub(super) async fn handle_classifier_misfire(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    name: &Mutex<String>,
) -> Result<String, String> {
    let nudge_hash = args
        .get("nudge_hash")
        .and_then(|v| v.as_str())
        .ok_or("missing 'nudge_hash' argument")?
        .to_string();
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    let agent = name.lock().await.clone();
    let body = serde_json::json!({
        "agent": agent,
        "nudge_hash": nudge_hash,
        "reason": reason,
    });
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/api/classifier-misfire");
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to record misfire: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("misfire failed ({status}): {body}"));
    }
    Ok("misfire recorded".to_string())
}

/// Agent-callable restart request. Fires a System-type message
/// via session.broadcast with a recognizable marker in the text. The
/// TUI event loop detects the marker and sets a pending-restart state;
/// Ryan clicks the resulting status-bar affordance to confirm.
///
/// The marker is a stable string (not user-visible) so the TUI can
/// pattern-match without depending on copy changes. The visible text
/// for Ryan is constructed TUI-side from the requester's role.
pub(super) const RESTART_REQUEST_MARKER: &str = "__BPMUX_RESTART_REQUEST__";

/// Marker that tells the TUI to open the tokens overlay. Same
/// delegation pattern as RESTART_REQUEST_MARKER — System-typed
/// broadcast with an opaque marker; TUI event loop intercepts and
/// calls the existing `handle_tokens` logic.
pub(super) const VIEW_TOKENS_MARKER: &str = "__BPMUX_VIEW_TOKENS__";

/// Marker for view_roadmap agent-triggered display.
pub(super) const VIEW_ROADMAP_MARKER: &str = "__BPMUX_VIEW_ROADMAP__";

pub(super) async fn handle_request_restart(
    ws_tx: &Mutex<Option<WsSink>>,
    name: &Mutex<String>,
) -> Result<String, String> {
    let requester = name.lock().await.clone();
    let short = requester
        .split(':')
        .next()
        .unwrap_or(&requester)
        .to_string();
    // System-type so it doesn't appear in chat history, only as a
    // signal the TUI processes. Text includes the marker + requester
    // role so the TUI knows who asked.
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: format!("{RESTART_REQUEST_MARKER}:{short}"),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        topic: None,
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("restart request send failed: {e}"))?;
    Ok(format!("restart requested by {short}"))
}

/// Agent-triggered view_tokens. Sends a system marker to the
/// TUI which opens the existing tokens overlay. Same delegation
/// pattern as request_restart.
pub(super) async fn handle_view_tokens(ws_tx: &Mutex<Option<WsSink>>) -> Result<String, String> {
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_TOKENS_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        topic: None,
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("view_tokens send failed: {e}"))?;
    Ok("token view opened".to_string())
}

/// Agent-triggered view_roadmap. Sends the system marker so the
/// TUI opens the overlay (Ryan-facing side effect) AND returns the
/// roadmap text to the caller so the agent can surface the content
/// in the conversation. Two audiences, one tool — without the
/// content return, an agent asked "what's on the roadmap?" couldn't
/// answer even though the TUI opens.
pub(super) async fn handle_view_roadmap(ws_tx: &Mutex<Option<WsSink>>) -> Result<String, String> {
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_ROADMAP_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        topic: None,
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("view_roadmap send failed: {e}"))?;
    // Return the actual content so the caller can share it in chat.
    // Fall through with a sparse-data-honest message when the file
    // is missing rather than silently returning empty.
    match crate::roadmap::fetch_roadmap() {
        Ok(text) => Ok(text),
        Err(e) => Ok(format!("roadmap view opened, but content unavailable: {e}")),
    }
}

// ── Story management tools (#140) ──────────────────────────────
// Server-side gh proxy with lane-aware permissions.

/// Agent-visible feature usage counts from the local session.
pub(super) async fn handle_get_usage(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("cannot determine home directory")?;
    let path = home.join(".bpmux").join("usage.json");
    match std::fs::read_to_string(&path) {
        Ok(contents) => Ok(contents),
        Err(_) => Ok("{}".to_string()),
    }
}
