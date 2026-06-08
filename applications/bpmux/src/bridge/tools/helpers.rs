use futures::SinkExt;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub(in crate::bridge) type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

pub(in crate::bridge) async fn ws_send(
    ws_tx: &Mutex<Option<WsSink>>,
    payload: String,
) -> Result<(), String> {
    let mut tx = ws_tx.lock().await;
    let tx = tx
        .as_mut()
        .ok_or("not connected to bpmux — reconnecting, try again in a few seconds")?;
    tx.send(tungstenite::Message::Text(payload.into()))
        .await
        .map_err(|e| format!("send failed: {e}"))
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

pub(super) fn identity_success(index: &crate::session::identity::IdentityIndex) -> String {
    let view = crate::bridge::view_model::IdentityIndexView::from(index);
    serde_json::to_string_pretty(&view).unwrap_or_else(|_| "{}".to_string())
}

pub(super) fn identity_error(e: &crate::session::identity::IdentityError) -> String {
    serde_json::json!({
        "error": e.kind(),
        "detail": e.to_string(),
    })
    .to_string()
}

pub(super) async fn get_session_dir() -> Result<std::path::PathBuf, String> {
    use crate::session::log::default_sessions_base;
    let base = default_sessions_base().map_err(|e| format!("sessions base: {e}"))?;
    let current = base.join("current");
    if current.is_symlink() {
        std::fs::read_link(&current).map_err(|e| format!("read current: {e}"))
    } else {
        Err("no current session".to_string())
    }
}
