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

pub(super) async fn post_simulate(
    port: u16,
    path: &str,
    body: &serde_json::Value,
) -> Result<String, String> {
    let token = load_auth_token()?;
    let url = crate::transport::local_url::api_url(port, path);
    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(&token)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
        .json(body)
        .send()
        .await
        .map_err(|e| format!("failed: {e}"))?;
    resp.text().await.map_err(|e| format!("failed: {e}"))
}

fn load_auth_token() -> Result<String, String> {
    let path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "auth token path unavailable (no home directory)".to_string())?;
    match crate::session::auth_token::read(&path) {
        Ok(t) if !t.is_empty() => Ok(t),
        Ok(_) => Err(format!(
            "Auth token not found at {}. Start bpmux first to generate it.",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(format!(
            "Auth token not found at {}. Start bpmux first to generate it.",
            path.display()
        )),
        Err(e) => Err(format!(
            "Auth token unreadable at {}. Check permissions. ({e})",
            path.display()
        )),
    }
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
