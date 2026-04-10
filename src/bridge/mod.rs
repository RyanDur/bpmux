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

mod tools;

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
    last_channel: Arc<Mutex<Option<Vec<String>>>>,
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
            last_channel: Arc::new(Mutex::new(None)),
        }
    }
}

async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Peer<RoleServer>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
    last_channel: Arc<Mutex<Option<Vec<String>>>>,
) {
    let ws_url = format!("ws://127.0.0.1:{port}/ws");
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

    if let Some(Ok(_welcome)) = ws_rx.next().await {
        eprintln!("Bridge: connected to bpmux on port {port}");
    }

    while let Some(Ok(frame)) = ws_rx.next().await {
        if let tungstenite::Message::Text(text) = frame {
            let msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            if let Ok(ServerMessage::Message {
                from,
                text,
                ts,
                to,
                message_type,
            }) = msg
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
                    Some(recipients) if recipients.iter().any(|r| r == "all") => true,
                    Some(recipients) => recipients.iter().any(|r| r == current_name.as_str()),
                };
                let channel = match &to {
                    None => "broadcast".to_string(),
                    Some(recipients) if recipients.iter().any(|r| r == "all") => {
                        "broadcast".to_string()
                    }
                    Some(recipients) => recipients.join(","),
                };
                drop(current_name);
                if !relevant {
                    continue;
                }

                // Store channel for reply-to-channel default
                if let Some(recipients) = &to {
                    let mut reply_to: Vec<String> = recipients.clone();
                    if !reply_to.contains(&from_short.to_string()) {
                        reply_to.push(from_short.to_string());
                    }
                    let current_name = name.lock().await;
                    reply_to.retain(|r| r != current_name.as_str());
                    drop(current_name);
                    *last_channel.lock().await = Some(reply_to);
                } else {
                    *last_channel.lock().await = None;
                }

                let notification = CustomNotification::new(
                    "notifications/claude/channel",
                    Some(json!({
                        "content": format!("{from_short}: {text}"),
                        "meta": {
                            "from": from,
                            "ts": ts,
                            "channel": channel,
                            "source": "bpmux"
                        }
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
        let last_channel = self.last_channel.clone();

        async move {
            *peer_store.lock().await = Some(peer.clone());

            tokio::spawn(async move {
                loop {
                    run_ws_connection(
                        port,
                        name.clone(),
                        peer.clone(),
                        ws_tx_store.clone(),
                        last_channel.clone(),
                    )
                    .await;
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
        let tools = vec![
            crate::mcp::tools::send_message_tool(
                "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
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
            crate::mcp::tools::simulate_click_tool(
                "Simulate a mouse click at (x, y) on the TUI. Use with capture_screen for interactive testing.",
            ),
            crate::mcp::tools::simulate_key_tool(
                "Simulate a key press on the TUI. Use with capture_screen for interactive testing.",
            ),
            crate::mcp::tools::set_name_tool(
                "Set your display name in the bpmux session. Call this once at the start to identify yourself (e.g., 'dev', 'pm').",
            ),
            crate::mcp::tools::set_status_tool(
                "Set your status in the bpmux session. available = green dot (can respond), busy = yellow dot (working), help = red dot (needs attention).",
            ),
        ];

        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let port = self.port;
        let ws_tx = self.ws_tx.clone();
        let name = self.name.clone();
        let last_channel = self.last_channel.clone();

        async move {
            let args = request.arguments.unwrap_or_default();
            let result = match request.name.as_ref() {
                "send_message" => tools::handle_send(&args, &ws_tx, &last_channel).await,
                "set_name" => tools::handle_set_name(&args, &name, &ws_tx).await,
                "set_status" => tools::handle_set_status(&args, &ws_tx).await,
                "read_messages" => tools::handle_read_messages(&args, port).await,
                "search_log" => tools::handle_search_log(&args, port).await,
                "list_lessons" => tools::handle_list_lessons(&args, port).await,
                "list_peer_lessons" => tools::handle_list_peer_lessons(&args, port).await,
                "save_lesson" => tools::handle_save_lesson(&args, port, &name).await,
                "capture_screen" => tools::handle_capture_screen(port).await,
                "simulate_click" => tools::handle_simulate_click(&args, port).await,
                "simulate_key" => tools::handle_simulate_key(&args, port).await,
                other => Err(format!("unknown tool: {other}")),
            };
            Ok(match result {
                Ok(text) => CallToolResult::success(vec![Content::text(text)]),
                Err(e) => CallToolResult::error(vec![Content::text(e)]),
            })
        }
    }
}

pub fn run_bridge(port: u16, name: &str) -> anyhow::Result<()> {
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
mod tests;
