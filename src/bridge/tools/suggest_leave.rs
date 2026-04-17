use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::bridge::tools_stories::{require_role, role_from_name};
use crate::transport::protocol::ClientMessage;

/// Marker the bridge emits when an agent calls suggest_leave (#273).
/// Kept in sync with SUGGEST_LEAVE_MARKER in terminal/event.rs.
/// Format: `__BPMUX_SUGGEST_LEAVE__|<thread_name>|<target_role>|<caller_role>|<reason>`
pub(in crate::bridge) const SUGGEST_LEAVE_MARKER: &str = "__BPMUX_SUGGEST_LEAVE__";

/// `suggest_leave` — soft-signal dismissal. Any participant suggests
/// another step out; target sees a non-modal banner and decides.
/// Reason is required — enforces context-carrying signal per
/// designer's spec (2026-04-17): no silent "you can go" dismissals.
pub(in crate::bridge) async fn handle_suggest_leave(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(
        agent_name,
        &["ryan", "pm", "designer", "dev"],
        "suggest_leave",
    )?;
    let thread = args
        .get("thread")
        .and_then(|v| v.as_str())
        .ok_or("missing 'thread' argument (thread name)")?;
    let target = args
        .get("target_role")
        .and_then(|v| v.as_str())
        .ok_or("missing 'target_role' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or("missing 'reason' argument (required — context is load-bearing)")?;
    let caller = role_from_name(agent_name);
    let marker = format!("{SUGGEST_LEAVE_MARKER}|{thread}|{target}|{caller}|{reason}");
    let msg = ClientMessage::Send {
        text: marker,
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        thread_id: None,
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("serialize: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("suggest_leave send failed: {e}"))?;

    let response = serde_json::json!({
        "thread": thread,
        "target": target,
        "caller": caller,
    });
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_prefix_is_distinguishable() {
        assert_ne!(SUGGEST_LEAVE_MARKER, "__BPMUX_THREAD_CREATE__");
        assert_ne!(SUGGEST_LEAVE_MARKER, "__BPMUX_THREAD_CLOSE__");
        assert!(SUGGEST_LEAVE_MARKER.starts_with("__BPMUX_"));
    }
}
