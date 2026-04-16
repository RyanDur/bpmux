use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

/// Agent-callable restart request. Fires a System-type message
/// via session.broadcast with a recognizable marker in the text. The
/// TUI event loop detects the marker and sets a pending-restart state;
/// Ryan clicks the resulting status-bar affordance to confirm.
///
/// The marker is a stable string (not user-visible) so the TUI can
/// pattern-match without depending on copy changes. The visible text
/// for Ryan is constructed TUI-side from the requester's role.
pub(in crate::bridge) const RESTART_REQUEST_MARKER: &str = "__BPMUX_RESTART_REQUEST__";

pub(in crate::bridge) async fn handle_request_restart(
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
