use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

/// Marker for view_roadmap agent-triggered display.
pub(in crate::bridge) const VIEW_ROADMAP_MARKER: &str = "__BPMUX_VIEW_ROADMAP__";

/// Agent-triggered view_roadmap. Sends the system marker so the
/// TUI opens the overlay (Ryan-facing side effect) AND returns the
/// roadmap text to the caller so the agent can surface the content
/// in the conversation. Two audiences, one tool — without the
/// content return, an agent asked "what's on the roadmap?" couldn't
/// answer even though the TUI opens.
pub(in crate::bridge) async fn handle_view_roadmap(
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_ROADMAP_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        topic: None,
        thread_id: None,
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
