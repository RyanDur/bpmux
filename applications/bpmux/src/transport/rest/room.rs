use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::json_headers;
use crate::room::LivePtyRegistry;

#[derive(serde::Deserialize)]
pub struct TerminalCloseRequest {
    pub terminal_id: String,
    #[serde(default)]
    pub force: bool,
}

pub async fn post_terminal_spawn(
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
) -> impl IntoResponse {
    let room_agent = std::env::var("BPMUX_ROOM_AGENT").ok();
    let shell = std::env::var("SHELL").ok();
    let (command, args) =
        crate::room::resolve_terminal_spawn(room_agent.as_deref(), shell.as_deref());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let integration = crate::room::osc7_shell_integration(&command).unwrap_or_default();
    let mut env: Vec<(&str, &str)> = vec![("TERM", "xterm-256color")];
    for (key, value) in &integration.env {
        env.push((key.as_str(), value.as_str()));
    }
    let spawned =
        crate::room::spawn_pty_retained(&command, &args, Some(std::path::Path::new(&home)), &env);
    drop(env);
    match spawned {
        Ok(mut live) => {
            if let Some(dir) = integration.zdotdir {
                live.attach_tempdir(dir);
            }
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

pub async fn get_terminals(
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
) -> impl IntoResponse {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let terminals: Vec<serde_json::Value> = pty_registry
        .reap_dead_and_list_live()
        .into_iter()
        .map(|terminal_id| serde_json::json!({ "terminal_id": terminal_id, "cwd": home }))
        .collect();
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "terminals": terminals }).to_string(),
    )
}

pub async fn post_terminal_close(
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
    axum::Json(req): axum::Json<TerminalCloseRequest>,
) -> impl IntoResponse {
    if !req.force && pty_registry.has_foreground_job(&req.terminal_id) == Some(true) {
        return (
            StatusCode::CONFLICT,
            json_headers(),
            serde_json::json!({ "needs_confirm": true }).to_string(),
        );
    }
    match pty_registry.remove(&req.terminal_id) {
        Some(live) => {
            live.close_and_reap();
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "closed": req.terminal_id }).to_string(),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            json_headers(),
            serde_json::json!({ "error": "no such terminal" }).to_string(),
        ),
    }
}
