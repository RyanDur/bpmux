use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::bridge::tools_stories::{require_role, role_from_name};
use crate::transport::protocol::ClientMessage;

/// Marker the bridge emits when an agent calls close_thread (#271).
/// Kept in sync with THREAD_CLOSE_MARKER in terminal/event.rs.
/// Format: `__BPMUX_THREAD_CLOSE__|<name>|<by_role>`
pub(in crate::bridge) const THREAD_CLOSE_MARKER: &str = "__BPMUX_THREAD_CLOSE__";

/// `close_thread` — flip the viewer's active thread back to broadcast
/// without mutating the underlying thread entity. Symmetric to
/// start_thread (view-only op, no deletion, no participant change).
pub(in crate::bridge) async fn handle_close_thread(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(
        agent_name,
        &["ryan", "pm", "designer", "dev"],
        "close_thread",
    )?;
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument")?;
    let by = role_from_name(agent_name);
    let marker = format!("{THREAD_CLOSE_MARKER}|{name}|{by}");
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
        .map_err(|e| format!("close_thread send failed: {e}"))?;

    let response = serde_json::json!({
        "name": name,
        "closed_by": by,
    });
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_prefix_is_distinguishable_from_create() {
        assert_ne!(THREAD_CLOSE_MARKER, "__BPMUX_THREAD_CREATE__");
        assert!(THREAD_CLOSE_MARKER.starts_with("__BPMUX_"));
    }
}
