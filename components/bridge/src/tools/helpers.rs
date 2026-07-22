use futures::SinkExt;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

pub async fn ws_send(ws_tx: &Mutex<Option<WsSink>>, payload: String) -> Result<(), String> {
    let mut tx = ws_tx.lock().await;
    let tx = tx
        .as_mut()
        .ok_or("not connected to bpmux — reconnecting, try again in a few seconds")?;
    tx.send(tungstenite::Message::Text(payload.into()))
        .await
        .map_err(|e| format!("send failed: {e}"))
}

pub(super) fn urlencoding(s: &str) -> String {
    local_url::percent_encode(s)
}

pub(super) fn identity_success(index: &memory::identity::IdentityIndex) -> String {
    let view = crate::view_model::IdentityIndexView::from(index);
    serde_json::to_string_pretty(&view).unwrap_or_else(|_| "{}".to_string())
}

pub(super) fn identity_error(e: &memory::identity::IdentityError) -> String {
    serde_json::json!({
        "error": e.kind(),
        "detail": e.to_string(),
    })
    .to_string()
}

pub(super) async fn get_data_dir() -> Result<std::path::PathBuf, String> {
    let base = location::resolve_default_base().map_err(|e| format!("data base: {e}"))?;
    location::durable_data_dir(&base).map_err(|e| format!("data dir: {e}"))
}
