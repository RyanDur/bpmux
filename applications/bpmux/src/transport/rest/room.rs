use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::json_headers;
use crate::room::LivePtyRegistry;

pub async fn post_terminal_spawn(
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
) -> impl IntoResponse {
    let room_agent = std::env::var("BPMUX_ROOM_AGENT").ok();
    let shell = std::env::var("SHELL").ok();
    let (command, args) =
        crate::room::resolve_terminal_spawn(room_agent.as_deref(), shell.as_deref());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    match crate::room::spawn_pty_retained(
        &command,
        &args,
        Some(std::path::Path::new(&home)),
        &[("TERM", "xterm-256color")],
    ) {
        Ok(live) => {
            let pid = live.pid();
            let terminal_id = uuid::Uuid::new_v4().to_string();
            pty_registry.insert(terminal_id.clone(), live);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "terminal_id": terminal_id, "pid": pid, "cwd": home })
                    .to_string(),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            serde_json::json!({ "error": format!("spawn failed: {e}") }).to_string(),
        ),
    }
}
