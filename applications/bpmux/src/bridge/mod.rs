//! Channel bridge: stdio MCP server that forwards to bpmux's WebSocket endpoint.
//!
//! Claude Code spawns this as a subprocess. It connects to a running bpmux instance
//! via WebSocket and bridges messages bidirectionally:
//!   - bpmux messages → Claude Code channel notifications (via stdout)
//!   - Claude Code tool calls → bpmux messages (via WebSocket)

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rmcp::model::{CustomNotification, ServerNotification};
use rmcp::service::Peer;
use rmcp::{ErrorData, RoleServer, ServiceExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub(crate) mod atp_substantive_gate;
pub(crate) mod deliver_story_body_diff_gate;
pub(crate) mod dispatch;
pub(crate) mod mcp_handler;
pub(crate) mod move_story;
pub(crate) mod polling;
mod rpc_client;
pub(crate) mod tool_registry;
pub(crate) mod tools;
pub(crate) mod tools_stories;
pub(crate) mod tracker;
pub(crate) mod view_model;

pub use tool_registry::{DISPATCHED_TOOL_NAMES, build_advertised_tools};

pub(crate) use polling::{
    drain_interval_from_env, hydration_interval_from_env, poll_interval_from_env, run_drain_loop,
};
pub use polling::{run_hydration_loop_cancellable, run_ingest_poll_loop_cancellable};
#[cfg(test)]
pub(crate) use polling::{
    run_ingest_poll_loop, select_stories_to_hydrate, select_stories_to_revalidate,
};

pub(crate) use dispatch::{
    dispatch_test_accept_story, dispatch_test_close_story, dispatch_test_comment_on_story,
    dispatch_test_create_iteration, dispatch_test_create_story, dispatch_test_deliver_story,
    dispatch_test_dev_in_flight, dispatch_test_edit_story, dispatch_test_fail_desk_check,
    dispatch_test_get_story, dispatch_test_label_story, dispatch_test_list_epic_children,
    dispatch_test_list_ready, dispatch_test_move_story, dispatch_test_pass_desk_check,
    dispatch_test_reject_story, dispatch_test_reorder_story, dispatch_test_set_parent_epic,
    dispatch_test_start_story, dispatch_test_unset_parent_epic, dispatch_test_view_backlog,
    dispatch_test_view_board, dispatch_test_view_comments, dispatch_test_view_story_timeline,
    dispatch_test_view_velocity,
};

#[cfg(test)]
pub(crate) mod test_fixtures;

/// Public re-export so the transport layer can fire the same board-
/// invalidate marker that MCP bridge tools use (#313). Keeping one
/// constant means the TUI event handler stays a single branch — no
/// separate matching paths for MCP-originated vs REST-originated
/// invalidates.
pub use tools::VIEW_BACKLOG_MARKER;
pub use tools_stories::BOARD_INVALIDATE_MARKER;
pub use tools_stories::SYNC_STATUS_UPDATE_MARKER;
pub use tools_stories::parse_acceptance_test_path;

use crate::transport::protocol::ServerMessage;

type McpError = ErrorData;
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tungstenite::Message,
>;

#[derive(Clone)]
pub struct BridgeServer {
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    ws_tx: Arc<Mutex<Option<WsSink>>>,
}

pub(crate) fn resolve_bridge_name(raw: &str) -> String {
    let is_placeholder = raw.starts_with("${") && raw.ends_with('}');
    if raw.is_empty() || raw == "claude-code" || is_placeholder {
        let suffix: String = uuid::Uuid::new_v4().to_string()[..4].to_string();
        format!("agent-{suffix}")
    } else {
        raw.to_string()
    }
}

impl BridgeServer {
    fn new(port: u16, name: String) -> Self {
        Self {
            port,
            name: Arc::new(Mutex::new(resolve_bridge_name(&name))),
            peer: Arc::new(Mutex::new(None)),
            ws_tx: Arc::new(Mutex::new(None)),
        }
    }
}

/// Validate a meta payload against the Claude Code channel contract.
///
/// Returns Err if any value is null. Claude Code v2.1.109+ rejects
/// notifications with null meta values, breaking the MCP stdio pipe
/// entirely (silent on v2.1.92, loud on later versions). This check
/// fails loudly at the boundary so we catch our own contract violations
/// before they cross into a system we don't control.
pub(super) fn validate_meta(
    meta: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    for (key, value) in meta {
        if value.is_null() {
            return Err(format!(
                "meta['{key}'] is null — Claude Code will reject the notification"
            ));
        }
    }
    Ok(())
}

/// Build the `meta` payload for a channel notification.
///
/// Optional fields are OMITTED (not sent as null) when absent. This matters
/// because:
/// - The MCP channel spec treats meta entries as tag attributes — null isn't a valid attribute
/// - Claude Code v2.1.109+ rejects notifications with null meta values outright
/// - Presence-as-signal semantics: "no addressee" (key omitted) means broadcast attention,
///   whereas "addressee: null" could be misread as "explicitly addressed to nobody"
///
/// Core fields (from/ts/channel/source) are always present. The two optional
/// fields (addressee/message_type) are only included when non-None.
pub(super) fn build_notification_meta(
    from: &str,
    ts: &str,
    channel: &str,
    addressee: Option<&str>,
    message_type: Option<&str>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut meta = serde_json::Map::new();
    meta.insert("from".to_string(), json!(from));
    meta.insert("ts".to_string(), json!(ts));
    meta.insert("channel".to_string(), json!(channel));
    meta.insert("source".to_string(), json!("bpmux"));
    if let Some(a) = addressee {
        meta.insert("addressee".to_string(), json!(a));
    }
    if let Some(mt) = message_type {
        meta.insert("message_type".to_string(), json!(mt));
    }
    meta
}

async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Peer<RoleServer>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
) {
    let ws_url = crate::transport::local_url::ws_url(port, "/ws");
    let (ws_stream, _) = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Bridge: failed to connect to bpmux WebSocket: {e}");
            return;
        }
    };

    let (ws_tx, mut ws_rx) = ws_stream.split();
    *ws_tx_store.lock().await = Some(ws_tx);

    {
        let current_name = name.lock().await.clone();
        let mut tx = ws_tx_store.lock().await;
        if let Some(ref mut tx) = *tx {
            let identify = json!({"type": "identify", "name": current_name, "tool": "claude-code"})
                .to_string();
            if let Err(e) = tx.send(tungstenite::Message::Text(identify.into())).await {
                eprintln!("Bridge: failed to identify: {e}");
                return;
            }
        }
    }

    match ws_rx.next().await {
        Some(Ok(_welcome)) => {
            eprintln!("Bridge: connected to bpmux on port {port}");
        }
        Some(Err(e)) => {
            eprintln!("Bridge: welcome frame error: {e}");
            return;
        }
        None => {
            eprintln!("Bridge: connection closed before welcome");
            return;
        }
    }

    while let Some(frame_result) = ws_rx.next().await {
        let frame = match frame_result {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Bridge: websocket receive error: {e}");
                break;
            }
        };
        let text = match frame {
            tungstenite::Message::Text(t) => t,
            tungstenite::Message::Close(_) => {
                eprintln!("Bridge: server closed connection");
                break;
            }
            tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => continue,
            other => {
                eprintln!(
                    "Bridge: unexpected frame type ({} bytes), dropping",
                    match &other {
                        tungstenite::Message::Binary(b) => b.len(),
                        _ => 0,
                    }
                );
                continue;
            }
        };
        let msg: ServerMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Bridge: failed to parse server message: {e} (text: {text})");
                continue;
            }
        };
        // Surface server-side errors so silent drops stop being silent.
        if let ServerMessage::Error { ref reason } = msg {
            eprintln!("Bridge: server rejected message: {reason}");
            continue;
        }
        if let ServerMessage::Message {
            from,
            text,
            ts,
            to,
            addressee,
            message_type,
            multi_depth: _,
            state_delta: _,
            state_snapshot: _,
        } = msg
        {
            let from_short = from.split(':').next().unwrap_or(&from);
            let current_name = name.lock().await;
            if from_short == *current_name {
                continue;
            }
            drop(current_name);

            let mt = message_type.as_deref();
            if mt == Some("activity") || mt == Some("system") {
                continue;
            }

            // Only deliver messages relevant to this agent
            let current_name = name.lock().await;
            let relevant = match &to {
                None => true,
                Some(recipients)
                    if recipients
                        .iter()
                        .any(|r| r == crate::session::message::BROADCAST_ALL) =>
                {
                    true
                }
                Some(recipients) => recipients.iter().any(|r| r == current_name.as_str()),
            };
            let channel = match &to {
                None => "broadcast".to_string(),
                Some(recipients)
                    if recipients
                        .iter()
                        .any(|r| r == crate::session::message::BROADCAST_ALL) =>
                {
                    "broadcast".to_string()
                }
                Some(recipients) => recipients.join(","),
            };
            drop(current_name);
            if !relevant {
                continue;
            }

            let meta = build_notification_meta(
                &from,
                &ts,
                &channel,
                addressee.as_deref(),
                message_type.as_deref(),
            );
            // Boundary validator: fail loudly if our payload violates the
            // Claude Code contract, rather than silently shipping it and
            // watching the MCP pipe crash.
            if let Err(violation) = validate_meta(&meta) {
                eprintln!("Bridge: CONTRACT VIOLATION on outbound notification: {violation}");
                continue;
            }
            let notification = CustomNotification::new(
                "notifications/claude/channel",
                Some(json!({
                    "content": format!("{from_short}: {text}"),
                    "meta": meta
                })),
            );

            if let Err(e) = peer
                .send_notification(ServerNotification::CustomNotification(notification))
                .await
            {
                eprintln!("Bridge: failed to send channel notification: {e}");
                break;
            }
        }
    }

    *ws_tx_store.lock().await = None;
    eprintln!("Bridge: disconnected from bpmux");
}

pub fn run_bridge(
    port: u16,
    name: &str,
    _fake_tracker: bool,
    _board_fixture: Option<String>,
) -> anyhow::Result<()> {
    let project_root = crate::workflow::load_project_root_or_cwd();
    let workflow = crate::workflow::load_workflow(&project_root).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
        )
    })?;
    crate::workflow::install_workflow(workflow);
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let server = BridgeServer::new(port, name.to_string());
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let service = server.serve((stdin, stdout)).await?;
        service.waiting().await?;
        anyhow::Ok(())
    })
}

#[cfg(test)]
mod poller_middleware_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_registry_tests;
#[cfg(test)]
mod tools_stories_tests;
#[cfg(test)]
mod tracker_contract_tests;
#[cfg(test)]
mod tracker_tests;
