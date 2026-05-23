use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

pub(in crate::bridge) const VIEW_TOKENS_MARKER: &str = "__BPMUX_VIEW_TOKENS__";

pub(in crate::bridge) async fn handle_view_tokens(
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_TOKENS_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("view_tokens send failed: {e}"))?;
    Ok("token view opened".to_string())
}
