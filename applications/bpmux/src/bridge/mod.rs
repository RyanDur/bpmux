//! Channel bridge: stdio MCP server that forwards to bpmux's WebSocket endpoint.
//!
//! Claude Code spawns this as a subprocess. It connects to a running bpmux instance
//! via WebSocket and bridges messages bidirectionally:
//!   - bpmux messages → Claude Code channel notifications (via stdout)
//!   - Claude Code tool calls → bpmux messages (via WebSocket)

use std::future::Future;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, CustomNotification, ListToolsResult,
    ServerCapabilities, ServerInfo, ServerNotification,
};
use rmcp::service::{NotificationContext, Peer, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub(crate) mod atp_substantive_gate;
pub(crate) mod deliver_story_body_diff_gate;
pub(crate) mod move_story;
mod rpc_client;
pub(crate) mod tools;
pub(crate) mod tools_stories;
pub(crate) mod tracker;
pub(crate) mod view_model;

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

/// #570 helper: broadcast a `ClientMessage::Send` workflow signal via
/// `session.broadcast` (canonical-store-side path). Used by dispatch_
/// test_* after handlers — handler-internal `ws_send(empty_ws_tx, ...)`
/// is silent post-#516 because RPC dispatch passes None ws_tx; this
/// helper restores the chat-side workflow-signal observation surface.
fn broadcast_client_msg_via_session(
    session: &Arc<crate::session::Session>,
    agent_name: &str,
    msg: crate::transport::protocol::ClientMessage,
) {
    if let crate::transport::protocol::ClientMessage::Send {
        text,
        to,
        addressee,
        message_type,
    } = msg
    {
        let explicit_privacy = to.is_some();
        let from_short = crate::transport::ws::short_name(agent_name);
        let session_msg = crate::session::Message {
            from: format!("{from_short}:claude-code"),
            text,
            ts: chrono::Utc::now(),
            message_type: crate::session::message::MessageType::from(message_type.as_deref()),
            to,
            addressee,
            explicit_privacy,
            ..crate::session::Message::default()
        };
        if let Err(e) = session.broadcast(&session_msg) {
            crate::session::diag::log_diag(&format!("rpc: workflow-signal broadcast failed: {e}"));
        }
    }
}

pub(crate) async fn dispatch_test_create_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_create_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_create_iteration(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_create_iteration(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_comment_on_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_comment_on_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_close_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_close_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_deliver_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_deliver_story(&args, agent_name, &empty_ws, refresh).await?;
    // #570: fire deliver_story workflow signals via session.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let title = refresh
            .and_then(|r| r.story(number))
            .map(|s| s.title)
            .unwrap_or_else(|| format!("#{number}"));
        for msg in tools_stories::build_delivery_signals(number, &title) {
            broadcast_client_msg_via_session(session, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_start_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_start_story(&args, agent_name, &empty_ws, refresh).await?;
    // #570: fire start_story workflow signal via session.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_start_signal(number),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_accept_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_accept_story(&args, agent_name, &empty_ws, refresh).await?;
    // #570: fire accept_story workflow signals via session.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let role = tools_stories::role_from_name(agent_name);
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_accept_signal(number, role),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_fail_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result =
        tools_stories::handle_fail_desk_check(&args, agent_name, &empty_ws, refresh).await?;
    // #570: fire fail_desk_check workflow signals via session.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("needs fixes");
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_fail_desk_check_signal(number, reason),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_pass_desk_check(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result =
        tools_stories::handle_pass_desk_check(&args, agent_name, &empty_ws, refresh).await?;
    // #570: handler's ws_send fired against empty ws_tx (silent
    // no-op post-#516); fire workflow signals via session.broadcast
    // here so observers actually see "ready for acceptance" in chat.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let story_type = tools_stories::classify_for_pass_desk_check(number, refresh);
        for msg in tools_stories::build_pass_desk_check_signals(number, story_type) {
            broadcast_client_msg_via_session(session, agent_name, msg);
        }
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_reject_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    let result = tools_stories::handle_reject_story(&args, agent_name, &empty_ws, refresh).await?;
    // #570: fire reject_story workflow signal via session.
    if let Some(number) = args.get("number").and_then(|v| v.as_u64()) {
        let reason = args
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("rejected");
        broadcast_client_msg_via_session(
            session,
            agent_name,
            tools_stories::build_reject_signal(number, reason),
        );
    }
    Ok(result)
}

pub(crate) async fn dispatch_test_label_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_label_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_set_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_set_parent_epic(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_unset_parent_epic(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_unset_parent_epic(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_reorder_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_reorder_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_move_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    move_story::handle_move_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_edit_story(
    args: serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools_stories::handle_edit_story(&args, agent_name, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_view_velocity(
    messages: Vec<crate::session::Message>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let empty_ws: tokio::sync::Mutex<Option<WsSink>> = tokio::sync::Mutex::new(None);
    tools::handle_view_velocity(|| async move { Ok(messages) }, &empty_ws, refresh).await
}

pub(crate) async fn dispatch_test_view_board(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    tools_stories::handle_view_board(&serde_json::Map::new(), refresh).await
}

pub(crate) async fn dispatch_test_get_story(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_get_story(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_list_ready(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "list_ready requires canonical store".to_string())?;
    tools::handle_list_ready(&store).await
}

pub(crate) async fn dispatch_test_list_epic_children(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "list_epic_children requires canonical store".to_string())?;
    tools::handle_list_epic_children(&args, &store).await
}

pub(crate) async fn dispatch_test_dev_in_flight(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "dev_in_flight requires canonical store".to_string())?;
    tools::handle_dev_in_flight(&store).await
}

pub(crate) async fn dispatch_test_view_story_timeline(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_story_timeline(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_view_comments(
    args: serde_json::Map<String, serde_json::Value>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh.map(|r| r.store());
    tools::handle_view_comments(&args, store.as_ref(), refresh).await
}

pub(crate) async fn dispatch_test_view_backlog(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let store = refresh
        .map(|r| r.store())
        .ok_or_else(|| "view_backlog requires canonical store".to_string())?;
    let stories = store.all_stories();
    let summary = tools::build_backlog_summary(&stories);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

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

impl ServerHandler for BridgeServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::builder().enable_tools().build();

        let mut experimental = std::collections::BTreeMap::new();
        experimental.insert("claude/channel".to_string(), serde_json::Map::new());
        capabilities.experimental = Some(experimental);

        ServerInfo::new(capabilities)
            .with_instructions("bpmux channel bridge — connects Claude Code to a bpmux session. The TUI is an 80x24 terminal. Keep messages concise — under 3 lines when possible. Send detailed content through your coding tool, use bpmux for coordination and short updates.")
    }

    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let peer = context.peer.clone();
        let port = self.port;
        let name = self.name.clone();
        let peer_store = self.peer.clone();
        let ws_tx_store = self.ws_tx.clone();

        async move {
            *peer_store.lock().await = Some(peer.clone());

            tokio::spawn(async move {
                loop {
                    run_ws_connection(port, name.clone(), peer.clone(), ws_tx_store.clone()).await;
                    eprintln!("Bridge: reconnecting in 3 seconds...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            });
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(
            ListToolsResult::with_all_items(build_advertised_tools()),
        ))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let port = self.port;
        let ws_tx = self.ws_tx.clone();
        let name = self.name.clone();

        async move {
            let args = request.arguments.unwrap_or_default();
            // Record every MCP tool invocation to the shared
            // usage store so team-judgment questions ("is this tool
            // used?") have data across agents and sessions. Single
            // choke-point; no per-handler instrumentation.
            crate::session::usage::record(
                crate::session::usage::Source::Mcp,
                request.name.as_ref(),
            );
            let result = match request.name.as_ref() {
                "send_message" => {
                    let current_name = name.lock().await.clone();
                    tools::handle_send(&args, &current_name, &ws_tx).await
                }
                "share_message" => {
                    let current_name = name.lock().await;
                    tools::handle_share_message(&args, port, &current_name, &ws_tx).await
                }
                "set_name" => tools::handle_set_name(&args, &name, &ws_tx).await,
                "set_status" => tools::handle_set_status(&args, &ws_tx).await,
                "read_messages" => {
                    let current_name = name.lock().await;
                    tools::handle_read_messages(&args, port, &current_name).await
                }
                "search_log" => tools::handle_search_log(&args, port).await,
                "list_lessons" => tools::handle_list_lessons(&args, port).await,
                "list_peer_lessons" => tools::handle_list_peer_lessons(&args, port).await,
                "save_lesson" => tools::handle_save_lesson(&args, port, &name).await,
                "capture_screen" => tools::handle_capture_screen(port).await,
                "capture_electron_window" => {
                    let role = name.lock().await.clone();
                    tools::handle_capture_electron_window(&role, port).await
                }
                "simulate_click" => tools::handle_simulate_click(&args, port).await,
                "simulate_key" => tools::handle_simulate_key(&args, port).await,
                "simulate_type" => tools::handle_simulate_type(&args, port).await,
                "simulate_input_clear" => tools::handle_simulate_input_clear(port).await,
                "simulate_input_submit" => tools::handle_simulate_input_submit(port).await,
                "classifier_misfire" => tools::handle_classifier_misfire(&args, port, &name).await,
                "save_bookmark" => {
                    let current_name = name.lock().await;
                    tools::handle_save_bookmark(&args, port, &current_name).await
                }
                "get_bookmark" => {
                    let current_name = name.lock().await;
                    tools::handle_get_bookmark(&args, port, &current_name).await
                }
                "save_briefing" => {
                    let current_name = name.lock().await;
                    tools::handle_save_briefing(&args, &current_name).await
                }
                "get_briefing" => {
                    let current_name = name.lock().await;
                    tools::handle_get_briefing(&args, &current_name).await
                }
                "get_identity" => {
                    let current_name = name.lock().await;
                    tools::handle_get_identity(&args, &current_name).await
                }
                "add_value" => {
                    let current_name = name.lock().await;
                    tools::handle_add_value(&args, &current_name).await
                }
                "add_principle" => {
                    let current_name = name.lock().await;
                    tools::handle_add_principle(&args, &current_name).await
                }
                "update_principle" => {
                    let current_name = name.lock().await;
                    tools::handle_update_principle(&args, &current_name).await
                }
                "remove_principle" => {
                    let current_name = name.lock().await;
                    tools::handle_remove_principle(&args, &current_name).await
                }
                "get_token_report" => tools::handle_get_token_report().await,
                "get_ci_state" => tools::handle_get_ci_state(&args, port).await,
                "get_ci_log_for_run" => tools::handle_get_ci_log_for_run(&args, port).await,
                "rerun_ci" => tools::handle_rerun_ci(&args, port).await,
                "user_fn" => {
                    let current_name = name.lock().await.clone();
                    tools::handle_user_fn(&args, &current_name).await
                }
                "list_user_fns" => {
                    let current_name = name.lock().await.clone();
                    tools::handle_list_user_fns(&args, &current_name).await
                }
                "get_sync_status" => tools::handle_get_sync_status(port).await,
                "view_story_commits" => tools::handle_view_story_commits(&args).await,
                "view_commit_diff" => tools::handle_view_commit_diff(&args).await,
                "view_velocity" => {
                    tools::handle_view_velocity(
                        || tools::fetch_messages_via_http(port),
                        &ws_tx,
                        None,
                    )
                    .await
                }
                "start_story"
                | "deliver_story"
                | "pass_desk_check"
                | "fail_desk_check"
                | "accept_story"
                | "reject_story"
                | "close_story"
                | "create_story"
                | "create_iteration"
                | "comment_on_story"
                | "label_story"
                | "move_story"
                | "set_parent_epic"
                | "unset_parent_epic"
                | "reorder_story"
                | "edit_story"
                | "view_board"
                | "get_story"
                | "list_ready"
                | "list_epic_children"
                | "dev_in_flight"
                | "view_story_timeline"
                | "view_comments"
                | "view_backlog" => {
                    let current_name = name.lock().await.clone();
                    rpc_client::dispatch_via_rpc(port, request.name.as_ref(), &current_name, &args)
                        .await
                }
                "close_iteration" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_close_iteration(&current_name, &args, &ws_tx, port).await
                }
                "get_usage" => tools::handle_get_usage(&args).await,
                "view_tokens" => tools::handle_view_tokens(&ws_tx).await,
                "view_roadmap" => tools::handle_view_roadmap(&ws_tx).await,
                other => Err(format!("unknown tool: {other}")),
            };
            Ok(match result {
                Ok(text) => CallToolResult::success(vec![Content::text(text)]),
                Err(e) => CallToolResult::error(vec![Content::text(e)]),
            })
        }
    }
}

/// Build the list of tools advertised via MCP `list_tools`. Extracted
/// from the trait body so the parity test in
/// `tests/mcp_advertisement_parity_test.rs` (#531) can compare the
/// advertised set against `DISPATCHED_TOOL_NAMES` without a full MCP
/// service stand-up. Adding a new tool here without a matching entry
/// in `DISPATCHED_TOOL_NAMES` (and a `call_tool` match arm) — or
/// vice-versa — fails the parity tests at unit-test time, which closes
/// the #530-shape gap structurally.
pub fn build_advertised_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::send_message_tool(
            "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
        ),
        crate::mcp::tools::share_message_tool(
            "Propagate a message to a wider scope with provenance marking. Two forms: (1) call with `scope` only to re-share your most recent visible chat message (adds `[Shared by <you> from <source> thread]` prefix); (2) call with `scope` and `text` to send fresh text with `[Shared by <you>]` prefix. Scope is `\"all\"` for broadcast or a comma-separated role list (e.g. `\"dev,pm\"`). Privacy gate: explicit-privacy messages (DMs) cannot be re-shared — paraphrase in your own words if they need wider visibility.",
        ),
        crate::mcp::tools::read_messages_tool("Read recent messages from the bpmux session."),
        crate::mcp::tools::search_log_tool(
            "Search the session log by keyword. Returns message clusters with context. Use to find team experiences, decisions, and discussion arcs.",
        ),
        crate::mcp::tools::list_lessons_tool(
            "List indexed experiences grouped by topic. Shows what the team has learned.",
        ),
        crate::mcp::tools::save_lesson_tool(
            "Save what you learned from an experience. Your lesson is tagged with your role and visible to the team.",
        ),
        crate::mcp::tools::list_peer_lessons_tool(
            "Read lessons from a connected peer. Returns the peer's lesson index (read-only). Use this to see what another machine has learned. Call save_lesson to internalize anything valuable.",
        ),
        crate::mcp::tools::capture_screen_tool(
            "Capture the current TUI screen as text. Returns the 80x24 grid exactly as the user sees it. Use for visual verification and desk checking.",
        ),
        crate::mcp::tools::capture_electron_window_tool(
            "Capture the bpmux Electron renderer window as a PNG (#1183). No args. Returns JSON `{ path, window_id, captured_at, width, height }` where `path` is a file under `~/.bpmux/captures/<timestamp>-<role>.png` that the agent can Read. macOS-only in v1: relies on system `screencapture` + `swift` (Xcode CLT). First-time use prompts macOS Screen Recording permission for the terminal/process running bpmux; until granted, the tool returns `{kind:'permission_denied', detail:...}` with the System Settings path. Other named error shapes: `electron_not_running` (no Electron process visible), `no_window` (Electron running but no bpmux window onscreen), `window_minimized` (window has 0x0 bounds). Captures are namespaced per role; the directory is auto-cleaned after 24h. Use this when you need to see what Ryan sees in the Electron app — replaces the Ryan-drops-screenshot anti-pattern.",
        ),
        crate::mcp::tools::simulate_click_tool(
            "Simulate a mouse click at (x, y) on the TUI. Use with capture_screen for interactive testing.",
        ),
        crate::mcp::tools::simulate_key_tool(
            "Simulate a key press on the TUI. Use with capture_screen for interactive testing.",
        ),
        crate::mcp::tools::simulate_type_tool(
            "Simulate multi-char typing into the current input buffer (e.g. slash commands like `/board` or DM text like `/to dev hello`). Same observable effect as Ryan typing each character. Pairs with simulate_input_submit to fire the Enter path.",
        ),
        crate::mcp::tools::simulate_input_clear_tool(
            "Clear the current input buffer. Use between simulate_type / simulate_key calls to reset state between audit scenarios. Takes no arguments.",
        ),
        crate::mcp::tools::simulate_input_submit_tool(
            "Submit the current input buffer — same effect as Ryan pressing Enter in Chat mode. Fires the command-or-send path (slash commands run, prose sends). Takes no arguments.",
        ),
        crate::mcp::tools::set_name_tool(
            "Set your display name in the bpmux session. Call this once at the start to identify yourself (e.g., 'dev', 'pm').",
        ),
        crate::mcp::tools::set_status_tool(
            "Set your status in the bpmux session. available = green dot (can respond), busy = yellow dot (working), help = red dot (needs attention).",
        ),
        crate::mcp::tools::classifier_misfire_tool(
            "Report a classifier nudge that fired on a message you judge is NOT a learning moment. The reason field is OPTIONAL — per the 'state observation, not conclusion' framing, simply marking a fire as wrong is valuable training data on its own. Use this whenever you see an experience-nudge in the chat that you reason internally is incorrect, so the team can measure classifier accuracy and tune it over time. See #159.",
        ),
        crate::mcp::tools::save_bookmark_tool(
            "Save a session bookmark — structured snapshot of your current state (task, decisions, open questions, next step). Survives compaction. Use before long operations or when you sense compaction is near.",
        ),
        crate::mcp::tools::get_bookmark_tool(
            "Read a session bookmark — the structured state snapshot saved by an agent. Use after compaction to resume context quickly.",
        ),
        crate::mcp::tools::save_briefing_tool(
            "#287: write a post-compact briefing that your future post-compact self reads on session-start. Free-form markdown capturing role-continuity — discipline reminders, active-work state, commitments to teammates, things you want to remember about who you are and what you're in the middle of. Distinct from bookmark (session work-state) and lessons (durable cross-session rules): briefings are cross-session but below-lesson-threshold, curated mid-iteration. Replaces any existing briefing for your role (max 1MB).",
        ),
        crate::mcp::tools::get_briefing_tool(
            "#287: read a post-compact briefing. Omit 'role' to read your own; pass a role name to inspect a peer's. Missing briefing returns empty string — not an error.",
        ),
        crate::mcp::tools::get_identity_tool(
            "Get an agent's identity — the stable values and principles that persist across sessions. Returns JSON with values, each carrying a description and a list of principles. Omit 'role' to get your own identity; pass a role name to see another agent's.",
        ),
        crate::mcp::tools::add_value_tool(
            "Create a new top-level value on your own identity (#269). Args: name + description. Rejects if a value with that name already exists (use update_principle / add_principle to amend rather than overwrite).",
        ),
        crate::mcp::tools::add_principle_tool(
            "Append a principle under one of your own identity values (#269). Role-derived: you can only modify your own identity. Args: value_name (e.g. 'correctness') + principle (the principle text). Rejects duplicates within a value. Returns the full post-state identity on success.",
        ),
        crate::mcp::tools::update_principle_tool(
            "Refine an existing principle's text on your own identity (#269). Args: value_name, old (current text), new (replacement text). Rejects if the old text isn't found exactly.",
        ),
        crate::mcp::tools::remove_principle_tool(
            "Remove a principle from your own identity (#269). Args: value_name + principle (exact text). Use to prune decoration after honest audit per the cost-to-hold discipline.",
        ),
        crate::mcp::tools::get_token_report_tool(
            "Get the token-cost report from bpmux's shared token log (#179). Returns JSON with total tokens, row count, per-role breakdown, and the 10 most recent turns (newest first). The data comes from per-harness adapters reading their own LLM-call recordings — you're seeing objective cost data, not self-reported. Same underlying log the `/tokens` TUI view reads, so you see exactly what Ryan sees. Empty report if the log is empty or missing.",
        ),
        crate::mcp::tools::get_ci_state_tool(
            "Returns the AGGREGATE CI conclusion for bpmux main branch across ALL workflows triggered by the latest commit (#1287). Returns JSON object: { conclusion: 'success'|'failure'|'in_progress'|'quota_exhausted'|'unknown', last_run_id, branch: 'main', url, head_sha, display_title, workflows: [{name, run_id, conclusion}, ...] }. `conclusion` is the cross-workflow aggregate: 'success' IFF all workflows on the latest commit succeeded; 'failure' IF any workflow failed; 'in_progress' IF any workflow is still running; 'quota_exhausted' IF any is quota-blocked and none red/running. `workflows` carries the per-run breakdown for diagnostic. `last_run_id` is the most-recent run's id (use `get_ci_log_for_run(run_id)` for per-run drill-in). Pull-based — agents call this on-demand for gate-fire decisions (pre-deliver, pre-desk-check, pre-accept). Any role can call.",
        ),
        crate::mcp::tools::get_ci_log_for_run_tool(
            "Returns structured per-test failure records for a specific GH Actions run id (#1174). Args: { run_id: int }. Returns JSON `{ run_id, conclusion, failures: [{ shard_id, test_name, file, line, error_message, retry_count }] }`. Use after `get_ci_state` reports `failure` and you have the `last_run_id` — closes the diagnostic gap that previously forced agents to ask Ryan for gh-web logs. Cached per run_id with TTL-forever for completed runs (append-only-immutable); in-progress runs return a named error shape `{ kind: 'run_in_progress', conclusion }` rather than empty results. Any role can call.",
        ),
        crate::mcp::tools::rerun_ci_tool(
            "Retrigger a failed GH Actions run via GitHub API direct (#1222). Args: { run_id: int }. Returns JSON `{ status: 'success' | 'failure', run_id, message? }`. Use this when a CI run failed on a transient flake (race-class, runner-unable-to-start, etc) to retrigger only the failed jobs without an empty-commit-as-CI-rerun anti-pattern and without falling back to gh CLI. NOT for code-broken runs — those need a substrate fix per `feedback_no_pre_push_hook_skip_ever`. Sister to get_ci_state. Any role can call.",
        ),
        crate::mcp::tools::user_fn_tool(
            "Invoke a per-role user-defined function (#459). Args: { name: string, args?: object }. Loads `bpmux-<role>/.bpmux/functions/<name>.md` (relative to bridge cwd; role-private — only your own role's directory), parses YAML frontmatter (name, description, type: user_fn, optional args[]), substitutes `{{key}}` slots in the body from args, returns JSON `{ status: 'ok' | 'not_defined' | 'missing_arg', name?, description?, body?, message? }`. Hot-reloads (no caching). Author definitions via Write/Edit (no separate authoring tool). v1 excludes composition / cross-role share / bpmux-side LLM invocation — your own Claude executes the returned body.",
        ),
        crate::mcp::tools::list_user_fns_tool(
            "List your role's defined user-fns (#459). Returns JSON `{ functions: [{ name, description, args? }, ...] }` sorted by name. Role-private — enumerates `bpmux-<role>/.bpmux/functions/*.md` for your own role's directory only.",
        ),
        crate::mcp::tools::get_sync_status_tool(
            "#451: inspect the board-sync orchestrator's current state. Returns JSON `{last_ingest_at, last_push_at, pending_events, last_error}` — the same observability surface the Electron app reads to show a source-down indicator. Use to diagnose why a mutation failed (refresh-before-write reports `source unreachable`), confirm a background ingest ran, or check the pending-push buffer depth. Any role can call.",
        ),
        crate::mcp::tools::get_story_tool(
            "Fetch full detail for a single story by issue number. Returns JSON `{number, title, body, state, labels[]}`. Any role can call. USE THIS INSTEAD OF `gh issue view <N>` — raw gh calls bypass the shared chat so teammates can't observe the read (#403 closes that bypass). Required read before pickup-gate paraphrasing a story.",
        ),
        crate::mcp::tools::view_comments_tool(
            "Fetch the full comment audit-trail for a story by issue number. Returns JSON array `[{id, author, body, created_at}, ...]` in chronological order. Any role can call. USE THIS INSTEAD OF `gh issue view <N> --json comments` — surfaces rejection-reason + filing-discussion + retro-scenarios for rework-pickup, desk-check context, and audit-trail re-audit without reaching outside bpmux.",
        ),
        crate::mcp::tools::dev_in_flight_tool(
            "Query whether dev currently has a story in the in-progress state. Returns JSON `{dev_in_flight: bool}` — a minimal yes/no read with no story identifiers so consumers (pm skills, interrupt-discipline checks) can't accidentally leak what dev is working on. Derived from server-side labels (in progress / started / finished) so the answer is authoritative even across bridge restarts. Any role can call.",
        ),
        crate::mcp::tools::list_ready_tool(
            "Enumerate the full ready queue. Returns JSON array of `{number, title, labels[]}` for every story labeled `ready`. Any role can call. USE THIS INSTEAD OF `gh issue list --label ready` — view_backlog surfaces counts + a sample, list_ready returns the complete queue for full pickup-gate visibility.",
        ),
        crate::mcp::tools::list_epic_children_tool(
            "List all stories whose `parent_epic` field equals the given Epic issue number. Returns JSON `{epic_number, children: [{number, title, state, story_type, is_closed}, ...]}` sorted by issue number. Any role can call. Use this for Epic-progress-grounding, count-divergence diagnosis, and ratchet-time enumeration. Errors: `epic_number not found` if the issue doesn't exist; `not an Epic` if the story_type isn't Epic.",
        ),
        crate::mcp::tools::view_board_tool(
            "View the project board — all stories grouped by state (icebox, ready, in progress, delivered, ready for accept, needs fixes, rejected, accepted) with counts. No host permission needed. Same data as the TUI board view.",
        ),
        crate::mcp::tools::view_velocity_tool(
            "View team velocity metrics (#209a) computed from workflow-signal timestamps over a rolling 14-day window. Returns JSON with delivered count, delivered-per-day, median pickup→deliver hours, median deliver→accept hours, rework rate, and a `sparse` flag when under-3-deliveries makes the medians indicative only. Objective data — no self-report. Also opens the velocity view in Ryan's TUI (project panel) so he can see it alongside your response — call this when he asks about velocity, not proactively.",
        ),
        crate::mcp::tools::create_story_tool(
            "Create a new story (GitHub issue). PM ONLY — dev/designer/ryan/other roles are rejected with a message redirecting to broadcast: surface the pain in chat and pm files the story. Story starts unlabeled in the icebox.",
        ),
        crate::mcp::tools::create_iteration_tool(
            "Create a new iteration-issue (#1297, Epic #1189 architectural-pivot slice 1). PM ONLY. theme_name surfaces as the GH issue title 'Iteration {theme_name}' and the issue carries the 'iteration' label so it decodes as story_type=Iteration. Sister to the Epic story_type — iteration-issues are the canonical substrate for iteration entities, replacing local markers-store. Rejects with named-collision error if an OPEN iteration with the same theme_name already exists (sister-discipline to #1281).",
        ),
        crate::mcp::tools::start_story_tool(
            "Mark a story as started. DEV ONLY — other roles are rejected with a clear message.",
        ),
        crate::mcp::tools::deliver_story_tool(
            "Mark a story as delivered. DEV ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::close_iteration_tool(
            "Close the current iteration: promote stories above the topmost iteration marker to ready, delete the consumed marker, broadcast the externalize-prompt (#282). PM ONLY. GUARDED (#1258 Ryan 2026-05-10 21:49 UTC): blocks with 409 if any non-icebox/non-accepted story exists in the pipeline (ready/in-progress/desk-check/delivered). Owner must accept or move_story-out all in-flight stories before close_iteration can fire.",
        ),
        crate::mcp::tools::label_story_tool(
            "Add a label to a story (ready, blocked, icebox, deferred). PM ONLY — other roles are rejected.",
        ),
        crate::mcp::tools::move_story_tool(
            "Move a story to a target state + optional position with required rationale (#1188). Single atomic mutation consolidates label_story / reorder_story / close_story / accept_story workflows. Args: number (issue), target_state (one of: icebox, ready, in-progress, desk-check, delivered, accepted, retired), position (optional 1-based; PM-only when set; only meaningful for ready/icebox), rationale (REQUIRED non-empty string — surfaced in story-timeline + appended as issue-comment). Closes today's #1177-stuck-Accepted class structurally: moving a closed story OUT of Accepted/Retired into a non-terminal column auto-fires StoryReopened first. Any role can move COLUMN; PM owns ORDER per Ryan 2026-05-05 01:39 UTC. Goes through canonical-store mutation chain (refresh → reap → state-transition → audit-comment) per `feedback_rest_mcp_canonical_mutation_parity` — no half-states observable.",
        ),
        crate::mcp::tools::set_parent_epic_tool(
            "Group a story under an Epic by setting its parent_epic. PM ONLY. Args: number (child) + epic_number (parent Epic). The Epic story-type is checked at filing-time on the parent; this tool only records the membership relationship. Self-parent rejected.",
        ),
        crate::mcp::tools::unset_parent_epic_tool(
            "Ungroup a story from its current Epic by clearing parent_epic. PM ONLY. Args: number (child). Idempotent — calling on a story not currently grouped succeeds silently.",
        ),
        crate::mcp::tools::close_story_tool(
            "Close a story with a reason (superseded / duplicate / won't-fix). Adds the reason as a comment on the issue + closes it. PM ONLY — accept/reject are separate paths for delivered stories. Use this for pre-delivery closures that don't route through the ship chain. Args: number + reason (required).",
        ),
        crate::mcp::tools::comment_on_story_tool(
            "Add a comment to a story. Durable issue-audit-trail — visible to any future reader of the issue. Distinct from bpmux chat (session-scoped). Use for post-filing context (reconsider-notes, cross-refs, pickup-gate observations) you want persistent. Any agent role. Args: number + body (required; markdown supported).",
        ),
        crate::mcp::tools::accept_story_tool(
            "Accept a delivered story and close it. PM + owner — PM fires after audit (pre-accept-quality-audit + pre-accept-ci-check + pre-accept-live-walk skills); owner fires when walking directly. Per Ryan 2026-05-14 22:24 UTC: PM has accept-fire authority.",
        ),
        crate::mcp::tools::reject_story_tool(
            "Reject a delivered story with a reason. PM + owner — PM fires when audit surfaces REJECT-flag (absolute-rule violation / falsifier-fidelity gap / substrate-correctness gap); owner fires when walking directly. Per Ryan 2026-05-14 22:24 UTC: PM has reject-fire authority. Rejected stories auto-flow to next iteration per `feedback_rejected_stories_default_to_next_iteration`.",
        ),
        crate::mcp::tools::reorder_story_tool(
            "Reorder a story's priority position within its state column. PM ONLY — other roles are rejected. Position 1 = top of column.",
        ),
        crate::mcp::tools::pass_desk_check_tool(
            "Mark a delivered story as passed desk-check. Applies the 'ready for accept' label and signals ryan that acceptance is ready. DESIGNER ONLY.",
        ),
        crate::mcp::tools::fail_desk_check_tool(
            "Mark a delivered story as failed desk-check with a reason. Applies the 'needs fixes' label and signals dev with the reason. DESIGNER ONLY.",
        ),
        crate::mcp::tools::view_tokens_tool(
            "#225: open the token-usage overlay in Ryan's TUI. Call when conversation warrants seeing token spend (e.g., 'how's our token usage?', 'are we over budget?'). The TUI shows the same data as the /tokens slash command; agents use this MCP tool instead of typing.",
        ),
        crate::mcp::tools::view_roadmap_tool(
            "#226: surface the roadmap in Ryan's TUI. Call when conversation warrants reviewing roadmap state (e.g., 'what's on the roadmap?', 'what are we working on next?'). Same content as the /roadmap slash command; agents use this MCP tool instead of typing.",
        ),
        crate::mcp::tools::view_backlog_tool(
            "#286: surface the board/backlog in Ryan's TUI. Call when conversation warrants reviewing the backlog state (e.g., 'how's the icebox?', 'what's in the ready queue?', 'where are we?'). Returns a JSON summary with counts + ready/delivered samples so the agent can answer inline AND opens the project panel for Ryan.",
        ),
        crate::mcp::tools::get_usage_tool(
            "Get feature usage statistics — invocation counts and last-used timestamps for all commands/features used this session. JSON format.",
        ),
        crate::mcp::tools::view_story_timeline_tool(
            "#418: fetch a story's complete gate-transition timeline — filed, pickup, delivered, desk-check PASS/FAIL (with reason), rejected (with reason), accepted — in one call. Each event carries timestamp + actor + payload. Sourced from structured marker comments (#391/#398/#419 gates) + story metadata. Any role can call. Use instead of scrolling chat or chaining view_comments + view_board to reconstruct a story's journey.",
        ),
        crate::mcp::tools::view_story_commits_tool(
            "#411: fetch the authoritative commit-set that landed a story. Returns JSON array of `{sha, short_sha, subject, author, committed_at}` for every commit on main whose subject references `#<N>` with a word boundary. Safer than `git log --grep` (body matches + number-prefix collisions). Any role can call. Use at desk-check to pin down what to code-review, at rework-pickup to find the prior-delivery baseline.",
        ),
        crate::mcp::tools::view_commit_diff_tool(
            "#412: read a commit's line-level diff inside bpmux. Accepts a sha/ref (or any git ref). Returns the output of `git show --format=fuller --patch <sha>` with an optional `max_bytes` cap (default 100000) — oversized diffs are truncated with a trailing marker citing shown/total bytes so the consumer knows to narrow the view. Any role can call. Use at desk-check to review implementation against spec without context-switching to terminal git-show.",
        ),
        crate::mcp::tools::edit_story_tool(
            "Replace a story's full markdown body. PM + Ryan only — other roles are rejected. Cascade: GitHub issue updated → canonical store refreshed via SourceAdapter::update_body → board-invalidate broadcast so renderers + bridges see the new body without restart. USE THIS INSTEAD OF `gh issue edit <N> --body` — raw gh calls bypass the canonical-store cascade so teammates can't observe the edit (#530). Args: number + body (full replacement; not a diff).",
        ),
    ]
}

/// Tool names that `BridgeServer::call_tool` dispatches. Mirror of the
/// match-arm name literals — kept as a const so the parity test in
/// `tests/mcp_advertisement_parity_test.rs` (#531) can compare it to
/// the advertised set returned by `build_advertised_tools`.
pub const DISPATCHED_TOOL_NAMES: &[&str] = &[
    "send_message",
    "share_message",
    "set_name",
    "set_status",
    "read_messages",
    "search_log",
    "list_lessons",
    "list_peer_lessons",
    "save_lesson",
    "capture_screen",
    "capture_electron_window",
    "simulate_click",
    "simulate_key",
    "simulate_type",
    "simulate_input_clear",
    "simulate_input_submit",
    "classifier_misfire",
    "save_bookmark",
    "get_bookmark",
    "save_briefing",
    "get_briefing",
    "get_identity",
    "add_value",
    "add_principle",
    "update_principle",
    "remove_principle",
    "get_token_report",
    "get_ci_state",
    "get_ci_log_for_run",
    "rerun_ci",
    "user_fn",
    "list_user_fns",
    "get_sync_status",
    "view_story_commits",
    "view_commit_diff",
    "view_velocity",
    "start_story",
    "deliver_story",
    "pass_desk_check",
    "fail_desk_check",
    "accept_story",
    "reject_story",
    "close_story",
    "create_story",
    "create_iteration",
    "comment_on_story",
    "label_story",
    "move_story",
    "set_parent_epic",
    "unset_parent_epic",
    "reorder_story",
    "edit_story",
    "view_board",
    "get_story",
    "list_ready",
    "list_epic_children",
    "dev_in_flight",
    "view_story_timeline",
    "view_comments",
    "view_backlog",
    "close_iteration",
    "get_usage",
    "view_tokens",
    "view_roadmap",
];

pub fn run_bridge(
    port: u16,
    name: &str,
    _fake_tracker: bool,
    _board_fixture: Option<String>,
) -> anyhow::Result<()> {
    let project_root = crate::workflow::load_project_root_or_cwd();
    let _workflow = crate::workflow::load_workflow(&project_root).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
        )
    })?;
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
pub(crate) async fn run_ingest_poll_loop(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    interval: std::time::Duration,
    observer: Option<crate::github::RateLimitObserver>,
) {
    run_ingest_poll_loop_cancellable(
        refresh,
        interval,
        observer,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
}

pub async fn run_ingest_poll_loop_cancellable(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    interval: std::time::Duration,
    observer: Option<crate::github::RateLimitObserver>,
    ct: tokio_util::sync::CancellationToken,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let base_backoff = std::time::Duration::from_secs(60);
    let max_backoff = std::time::Duration::from_secs(3600);
    let mut backoff = base_backoff;
    let mut last_poll_since: Option<chrono::DateTime<chrono::Utc>> = None;
    loop {
        tokio::select! {
            _ = ct.cancelled() => break,
            _ = ticker.tick() => {}
        }
        let poll_started_at = chrono::Utc::now();
        match refresh.refresh_all_since(last_poll_since).await {
            Ok(()) => {
                backoff = base_backoff;
                last_poll_since = Some(poll_started_at);
            }
            Err(e) => {
                let msg = e.to_string();
                let snapshot = observer.as_ref().map(|o| o.snapshot()).unwrap_or_default();
                if let Some(decision) = crate::github::classify_and_decide(
                    &msg,
                    snapshot,
                    backoff,
                    std::time::SystemTime::now(),
                ) {
                    crate::session::diag::log_diag(&format!(
                        "bridge: ingest rate-limited; backoff signal={} wait={}s",
                        crate::github::signal_label(decision.signal),
                        decision.wait.as_secs()
                    ));
                    tokio::time::sleep(decision.wait).await;
                    backoff = (backoff * 2).min(max_backoff);
                } else {
                    crate::session::diag::log_diag(&format!(
                        "bridge: background ingest poll failed: {e}"
                    ));
                }
            }
        }
    }
}

pub(crate) fn poll_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_POLL_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    std::time::Duration::from_secs(secs.max(1))
}

const HYDRATION_MAX_PER_TICK: usize = 3;

pub(crate) fn select_stories_to_hydrate(stories: &[crate::board::Story], limit: usize) -> Vec<u64> {
    use crate::board::domain::StoryState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                s.state,
                StoryState::Ready
                    | StoryState::InProgress
                    | StoryState::DeskCheck
                    | StoryState::Delivered
            ) && s.body.is_none()
        })
        .take(limit)
        .map(|s| s.number)
        .collect()
}

pub(crate) fn select_stories_to_revalidate(
    stories: &[crate::board::Story],
    limit: usize,
) -> Vec<(u64, Option<String>)> {
    use crate::board::domain::StoryState;
    stories
        .iter()
        .filter(|s| {
            matches!(
                s.state,
                StoryState::Ready
                    | StoryState::InProgress
                    | StoryState::DeskCheck
                    | StoryState::Delivered
            ) && s.body.is_some()
        })
        .take(limit)
        .map(|s| (s.number, s.body_etag.clone()))
        .collect()
}

pub async fn run_hydration_loop_cancellable(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    interval: std::time::Duration,
    observer: crate::github::RateLimitObserver,
    ct: tokio_util::sync::CancellationToken,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ct.cancelled() => break,
            _ = ticker.tick() => {}
        }
        let snapshot = observer.snapshot();
        let decision = crate::github::decide_backoff(
            snapshot,
            std::time::Duration::ZERO,
            std::time::SystemTime::now(),
        );
        if decision.wait > std::time::Duration::ZERO {
            tokio::select! {
                _ = ct.cancelled() => break,
                _ = tokio::time::sleep(decision.wait) => {}
            }
            continue;
        }
        let stories = refresh.all_stories();
        let to_hydrate = select_stories_to_hydrate(&stories, HYDRATION_MAX_PER_TICK);
        let to_revalidate = select_stories_to_revalidate(&stories, HYDRATION_MAX_PER_TICK);
        if to_hydrate.is_empty() && to_revalidate.is_empty() {
            crate::session::diag::log_diag(
                "bridge: hydration tick: nothing to hydrate or revalidate",
            );
            continue;
        }
        crate::session::diag::log_diag(&format!(
            "bridge: hydration tick: hydrate={:?} revalidate={:?}",
            to_hydrate,
            to_revalidate.iter().map(|(n, _)| *n).collect::<Vec<_>>()
        ));
        let mut hydrated_ok = 0u32;
        let mut revalidated_ok = 0u32;
        let to_hydrate_len = to_hydrate.len();
        for (idx, (number, prior_etag)) in to_hydrate
            .into_iter()
            .map(|n| (n, None::<String>))
            .chain(to_revalidate)
            .enumerate()
        {
            match refresh.fetch_body_conditional(number, prior_etag).await {
                Ok(_) => {
                    if idx < to_hydrate_len {
                        hydrated_ok += 1;
                    } else {
                        revalidated_ok += 1;
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "bridge: hydrate body #{number} failed: {e}"
                    ));
                    break;
                }
            }
        }
        crate::session::diag::log_diag(&format!(
            "bridge: hydration tick complete: hydrated={hydrated_ok} revalidated={revalidated_ok}"
        ));
    }
}

pub(crate) fn hydration_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_HYDRATION_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(15);
    std::time::Duration::from_secs(secs.max(1))
}

pub(crate) async fn run_drain_loop(
    refresh: Arc<dyn crate::board::RefreshDriver>,
    session: Arc<crate::session::Session>,
    interval: std::time::Duration,
) {
    let start = tokio::time::Instant::now() + interval;
    let mut ticker = tokio::time::interval_at(start, interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let state_changed = match refresh.drain_pending().await {
            Ok(0) => false,
            Ok(n) => {
                crate::session::diag::log_diag(&format!(
                    "bridge: background drain pushed {n} event(s)"
                ));
                true
            }
            Err(e) => {
                crate::session::diag::log_diag(&format!("bridge: background drain: {e}"));
                true
            }
        };
        if state_changed {
            broadcast_sync_status_update(&session);
        }
    }
}

fn broadcast_sync_status_update(session: &crate::session::Session) {
    let msg = crate::session::Message {
        from: "bpmux".to_string(),
        text: SYNC_STATUS_UPDATE_MARKER.to_string(),
        ts: chrono::Utc::now(),
        message_type: crate::session::MessageType::System,
        ..crate::session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::session::diag::log_diag(&format!(
            "run_drain_loop: sync-status marker broadcast failed: {e}"
        ));
    }
}

pub(crate) fn drain_interval_from_env() -> std::time::Duration {
    let secs = std::env::var("BPMUX_DRAIN_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    std::time::Duration::from_secs(secs.max(1))
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
