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
    // Forward `modifiers` if the caller supplied it (#170). Accept an
    // array of strings; let the REST endpoint validate names so the
    // bridge doesn't need to know the modifier vocabulary.
    let mut body = serde_json::json!({"key": key});
    if let Some(modifiers) = args.get("modifiers") {
        body["modifiers"] = modifiers.clone();
    }
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

/// `save_bookmark` (#168) — saves session state for post-compaction recovery.
pub(super) async fn handle_save_bookmark(
    args: &serde_json::Map<String, serde_json::Value>,
    _port: u16,
    name: &str,
) -> Result<String, String> {
    use crate::session::bookmark::{Bookmark, save_bookmark};
    // Get the session dir from the running instance via the screen endpoint.
    // Simpler: use the session dir from the REST messages path.
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

/// `get_bookmark` (#168) — reads a session bookmark.
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

/// Get the session directory path from the running bpmux instance.
async fn get_session_dir() -> Result<std::path::PathBuf, String> {
    // Read a message to discover the session dir — the REST endpoint
    // knows the session. Simpler approach: query the messages endpoint
    // and infer from the response. But actually, the session dir is
    // available via the state. For now, use the default sessions base
    // and find the current symlink.
    use crate::session::log::default_sessions_base;
    let base = default_sessions_base().map_err(|e| format!("sessions base: {e}"))?;
    let current = base.join("current");
    if current.is_symlink() {
        std::fs::read_link(&current).map_err(|e| format!("read current: {e}"))
    } else {
        Err("no current session".to_string())
    }
}

/// `get_identity` (#190) — reads the identity file for a role.
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
    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("failed to record misfire: {e}"))?;
    Ok("misfire recorded".to_string())
}

// ── Story management tools (#140) ──────────────────────────────
// Server-side gh proxy with lane-aware permissions.

/// Compile-time repo root — guaranteed to be the bpmux source
/// directory regardless of which workspace starts the bridge.
const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Run a gh command and return stdout. Errors include stderr.
/// Sets the working directory to the bpmux repo so `gh` can find
/// git remotes even when the bridge runs from an agent workspace.
async fn gh(args: &[&str]) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("gh");
    cmd.args(args);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    } else {
        // Use the compile-time repo root so gh finds git remotes
        // regardless of the bridge's runtime working directory.
        cmd.current_dir(REPO_ROOT);
    }
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output())
        .await
        .map_err(|_| "gh command timed out after 30s".to_string())?
        .map_err(|e| format!("gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract the short role name from the agent's identity.
/// "dev:claude-code" → "dev", "pm" → "pm".
fn role_from_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

/// Lane check: returns Ok(()) if allowed, Err with rejection message if not.
fn require_role(agent_name: &str, allowed: &[&str], operation: &str) -> Result<(), String> {
    let role = role_from_name(agent_name);
    if allowed.contains(&role) {
        Ok(())
    } else {
        Err(format!(
            "{operation} is restricted to {}. ask {} to {operation}.",
            allowed.join(" or "),
            allowed[0],
        ))
    }
}

/// View board — any agent, no lane restriction.
pub(super) async fn handle_view_board(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let json_str = gh(&[
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "200",
        "--json",
        "number,title,state,labels",
    ])
    .await?;

    let issues: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| format!("parse: {e}"))?;

    let mut ready = Vec::new();
    let mut in_progress = Vec::new();
    let mut delivered = Vec::new();
    let mut accepted = Vec::new();
    let mut deferred = Vec::new();
    let mut icebox = Vec::new();

    for issue in &issues {
        let num = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?");
        let gh_state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        let entry = format!("#{num} {title}");
        let has = |name: &str| labels.contains(&name);

        if gh_state == "CLOSED" {
            accepted.push(entry);
        } else if has("deferred") {
            deferred.push(entry);
        } else if has("delivered") {
            delivered.push(entry);
        } else if has("started") || has("finished") {
            in_progress.push(entry);
        } else if has("ready") {
            ready.push(entry);
        } else if has("icebox") {
            icebox.push(entry);
        } else {
            icebox.push(entry);
        }
    }

    let mut out = String::new();
    let section = |name: &str, items: &[String], out: &mut String| {
        out.push_str(&format!("{name} ({})\n", items.len()));
        for item in items {
            out.push_str(&format!("  {item}\n"));
        }
        out.push('\n');
    };
    section("READY", &ready, &mut out);
    section("IN PROGRESS", &in_progress, &mut out);
    section("DELIVERED", &delivered, &mut out);
    section("ACCEPTED", &accepted, &mut out);
    section("DEFERRED", &deferred, &mut out);
    section("ICEBOX", &icebox, &mut out);
    Ok(out)
}

/// Create a story — any agent can create.
pub(super) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd_args = vec!["issue", "create", "--title", title];
    if !body.is_empty() {
        cmd_args.extend(["--body", body]);
    }
    let result = gh(&cmd_args).await?;
    Ok(format!("story created: {}", result.trim()))
}

/// Start a story — dev only.
pub(super) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "start")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--add-label",
        "started",
    ])
    .await?;
    Ok(format!("#{number} marked as started"))
}

/// Deliver a story — dev only.
pub(super) async fn handle_deliver_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "deliver")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    // Remove started, add delivered
    gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "started",
        "--add-label",
        "delivered",
    ])
    .await?;
    Ok(format!("#{number} marked as delivered"))
}

/// Add/change labels — PM only.
pub(super) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "label")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or("missing 'label' argument")?;
    gh(&["issue", "edit", &number.to_string(), "--add-label", label]).await?;
    Ok(format!("#{number} labeled '{label}'"))
}

/// Accept a story — owner (ryan) only.
pub(super) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["ryan"], "accept")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    // Remove delivered, close the issue
    gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "delivered",
    ])
    .await?;
    gh(&["issue", "close", &number.to_string()]).await?;
    Ok(format!("#{number} accepted and closed"))
}

/// Reject a story — owner (ryan) only.
pub(super) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["ryan"], "reject")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("rejected");
    // Remove delivered, add rejected label, add comment
    gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "delivered",
        "--add-label",
        "rejected",
    ])
    .await?;
    gh(&["issue", "comment", &number.to_string(), "--body", reason]).await?;
    Ok(format!("#{number} rejected: {reason}"))
}

/// Reorder a story within its state column — PM only (#194).
pub(super) async fn handle_reorder_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "reorder")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as usize;
    let state = args
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'state' argument (e.g., 'ready', 'in_progress', 'delivered')")?;

    let path = crate::session::priority::default_priority_path()
        .map_err(|e| format!("priority path: {e}"))?;
    let mut index = crate::session::priority::load_priority(&path);
    crate::session::priority::reorder(&mut index, state, number, position);
    crate::session::priority::save_priority(&path, &index)
        .map_err(|e| format!("save priority: {e}"))?;
    Ok(format!("#{number} moved to position {position} in {state}"))
}

/// Get usage statistics — any agent (#195).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_from_name_extracts_short_role() {
        assert_eq!(role_from_name("dev:claude-code"), "dev");
        assert_eq!(role_from_name("pm"), "pm");
        assert_eq!(role_from_name("designer:cursor"), "designer");
        assert_eq!(role_from_name("ryan:human"), "ryan");
    }

    #[test]
    fn require_role_allows_matching_role() {
        assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
        assert!(require_role("pm", &["pm"], "label").is_ok());
        assert!(require_role("ryan:human", &["ryan"], "accept").is_ok());
    }

    #[test]
    fn require_role_rejects_non_matching_role() {
        let result = require_role("designer:claude-code", &["dev"], "start");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to dev"));
    }

    #[test]
    fn require_role_rejects_pm_from_dev_operations() {
        let result = require_role("pm:claude-code", &["dev"], "deliver");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to dev"));
    }

    #[test]
    fn require_role_rejects_dev_from_pm_operations() {
        let result = require_role("dev:claude-code", &["pm"], "label");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to pm"));
    }

    #[test]
    fn require_role_rejects_agents_from_owner_operations() {
        let result = require_role("dev:claude-code", &["ryan"], "accept");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to ryan"));

        let result = require_role("pm:claude-code", &["ryan"], "reject");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to ryan"));
    }
}
