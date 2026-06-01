use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::json_headers;
use crate::room::LivePtyRegistry;

pub async fn post_room_spawn(
    Path(role): Path<String>,
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
) -> impl IntoResponse {
    if pty_registry.contains(&role) {
        return (
            StatusCode::OK,
            json_headers(),
            r#"{"spawned":false,"already":true}"#.to_string(),
        );
    }
    let room_agent = std::env::var("BPMUX_ROOM_AGENT").ok();
    let shell = std::env::var("SHELL").ok();
    let (command, args) =
        crate::room::resolve_terminal_spawn(room_agent.as_deref(), shell.as_deref());
    let home = std::env::var("HOME").ok();
    match crate::room::spawn_pty_retained(
        &command,
        &args,
        home.as_deref().map(std::path::Path::new),
        &[("TERM", "xterm-256color")],
    ) {
        Ok(live) => {
            let pid = live.pid();
            pty_registry.insert(role, live);
            (
                StatusCode::OK,
                json_headers(),
                format!("{{\"spawned\":true,\"pid\":{pid}}}"),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            format!("{{\"error\":\"spawn failed: {e}\"}}"),
        ),
    }
}
