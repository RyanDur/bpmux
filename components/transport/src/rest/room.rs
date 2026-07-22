use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use super::shared::json_headers;
use room::LivePtyRegistry;
use store::{Dispatch, Select};

#[derive(serde::Deserialize)]
pub struct TerminalCloseRequest {
    pub terminal_id: String,
    #[serde(default)]
    pub force: bool,
}

pub async fn post_terminal_spawn(
    Extension(pty_registry): Extension<Arc<LivePtyRegistry>>,
    Extension(port): Extension<u16>,
    Extension(dispatch): Extension<Dispatch>,
    Extension(select): Extension<Select>,
    State(hub): State<Arc<hub::Hub>>,
) -> impl IntoResponse {
    let room_agent = std::env::var("BPMUX_ROOM_AGENT").ok();
    let shell = std::env::var("SHELL").ok();
    let (command, args) = room::resolve_terminal_spawn(room_agent.as_deref(), shell.as_deref());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let terminal_id = uuid::Uuid::new_v4().to_string();
    let binary = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "bpmux".to_string());
    let bridge = room::BridgeWiring {
        binary,
        port,
        agent_name: user_routing::bridge_agent_name(&terminal_id),
    };
    let integration = room::terminal_provisioning(&command, Some(&bridge), &room::owner_zdotdir())
        .unwrap_or_default();
    let port_str = port.to_string();
    let mut env: Vec<(&str, &str)> = vec![
        ("TERM", "xterm-256color"),
        ("BPMUX_TERMINAL_ID", terminal_id.as_str()),
        ("BPMUX_PORT", port_str.as_str()),
    ];
    for (key, value) in &integration.env {
        env.push((key.as_str(), value.as_str()));
    }
    let spawn_args: &[String] = if integration.args.is_empty() {
        &args
    } else {
        &integration.args
    };
    let spawned = room::spawn_pty_retained(
        &command,
        spawn_args,
        Some(std::path::Path::new(&home)),
        &env,
    );
    drop(env);
    match spawned {
        Ok(mut live) => {
            if let Some(dir) = integration.zdotdir {
                live.attach_tempdir(dir);
            }
            let pid = live.pid();
            pty_registry.insert(terminal_id.clone(), live);
            if let Some(output) = pty_registry.subscribe_for(&terminal_id) {
                room::spawn_agent_run_detector(
                    terminal_id.clone(),
                    output,
                    pty_registry.clone(),
                    dispatch.clone(),
                    select.clone(),
                    hub.clone(),
                );
            }
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
            if let Err(e) = tokio::task::spawn_blocking(move || live.close_and_reap()).await {
                diag::log_diag(&format!(
                    "terminal close: reap task for {} did not complete: {e}",
                    req.terminal_id
                ));
            }
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
