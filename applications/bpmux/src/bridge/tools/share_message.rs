use tokio::sync::Mutex;

use super::helpers::{WsSink, urlencoding, ws_send};
use crate::transport::protocol::ClientMessage;

/// Parse the `scope` arg into a target-recipient list. "all" (the
/// BROADCAST_ALL sentinel) widens to everyone and returns None;
/// other strings are split on comma into explicit recipients.
/// Returns Err when the arg is empty or only whitespace.
pub(in crate::bridge) fn parse_share_scope(scope: &str) -> Result<Option<Vec<String>>, String> {
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
pub(in crate::bridge) fn format_share_marker(
    agent: &str,
    source_scope: Option<&str>,
    text: &str,
) -> String {
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
pub(in crate::bridge) fn pick_most_recent_visible_chat(
    messages: &[serde_json::Value],
) -> Option<&serde_json::Value> {
    messages.iter().find(|m| {
        // Chat is represented by missing or null message_type in
        // the REST output (the default variant). Any explicit
        // message_type string is a non-chat kind we should skip.
        m.get("message_type").map(|v| v.is_null()).unwrap_or(true)
    })
}

pub(in crate::bridge) fn build_reshare_message(
    source: &serde_json::Value,
    agent_short: &str,
    target_to: Option<Vec<String>>,
) -> Result<ClientMessage, String> {
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
            let sender_short = source
                .get("from")
                .and_then(|v| v.as_str())
                .and_then(|s| s.split(':').next())
                .unwrap_or("peer");
            sender_short.to_string()
        }
    };
    let marker = format_share_marker(agent_short, Some(&source_scope), source_text);
    Ok(ClientMessage::Send {
        text: marker,
        to: target_to,
        addressee: None,
        message_type: None,
    })
}

/// `share_message` tool — widen a recent message (Form 1) or send
/// fresh text (Form 2) to a wider scope with provenance marking.
/// Mirrors the TUI's `/share` semantic for agents, including the
/// explicit-privacy refusal: messages the source sender marked
/// private cannot be re-shared (agents must paraphrase in their
/// own words if they need wider visibility).
pub(in crate::bridge) async fn handle_share_message(
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
    let url = crate::transport::local_url::api_url(
        port,
        &format!("/api/messages?limit=20&as={as_encoded}"),
    );
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

    let json = serde_json::to_string(&build_reshare_message(source, short, target_to)?)
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
