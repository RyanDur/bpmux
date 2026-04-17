use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::bridge::tools_stories::{require_role, role_from_name};
use crate::transport::protocol::ClientMessage;

/// Marker the bridge emits when an agent accepts an invitation (#274).
/// Kept in sync with THREAD_JOIN_MARKER in terminal/event.rs.
/// Format: `__BPMUX_THREAD_JOIN__|<thread>|<joining_role>`
pub(in crate::bridge) const THREAD_JOIN_MARKER: &str = "__BPMUX_THREAD_JOIN__";

/// `join_thread` — accept a pull_into_thread invitation. Mirror of
/// start_thread's view-switch mechanic but operates on an existing
/// thread: adds caller to participants, switches their TUI into the
/// thread view, and announces the join to existing participants.
pub(in crate::bridge) async fn handle_join_thread(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(
        agent_name,
        &["ryan", "pm", "designer", "dev"],
        "join_thread",
    )?;
    let thread = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument (thread name to join)")?;
    let joiner = role_from_name(agent_name);
    let marker = format!("{THREAD_JOIN_MARKER}|{thread}|{joiner}");
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
        .map_err(|e| format!("join_thread send failed: {e}"))?;

    let response = serde_json::json!({
        "thread": thread,
        "joined_by": joiner,
    });
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_prefix_is_distinguishable() {
        assert_ne!(THREAD_JOIN_MARKER, "__BPMUX_THREAD_CREATE__");
        assert_ne!(THREAD_JOIN_MARKER, "__BPMUX_PULL_INTO_THREAD__");
        assert!(THREAD_JOIN_MARKER.starts_with("__BPMUX_"));
    }
}
