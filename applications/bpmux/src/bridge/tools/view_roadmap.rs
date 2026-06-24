use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

pub(in crate::bridge) const VIEW_ROADMAP_MARKER: &str = "__BPMUX_VIEW_ROADMAP__";

pub(in crate::bridge) async fn handle_view_roadmap(
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_ROADMAP_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("view_roadmap send failed: {e}"))?;
    match crate::product_management::roadmap::fetch_roadmap() {
        Ok(text) => Ok(text),
        Err(e) => Ok(format!("roadmap view opened, but content unavailable: {e}")),
    }
}
