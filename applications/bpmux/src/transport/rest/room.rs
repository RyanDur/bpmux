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
    let agent = std::env::var("BPMUX_ROOM_AGENT").unwrap_or_else(|_| "claude-code".to_string());
    match crate::room::spawn_pty_retained(&agent, &[]) {
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
