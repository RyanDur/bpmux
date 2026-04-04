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
    ServerCapabilities, ServerInfo, ServerNotification, Tool,
};
use rmcp::service::{NotificationContext, Peer, RequestContext};
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

use crate::transport::protocol::{ClientMessage, ServerMessage};

type McpError = ErrorData;
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tungstenite::Message,
>;

#[derive(Clone)]
pub struct BridgeServer {
    port: u16,
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    ws_tx: Arc<Mutex<Option<WsSink>>>,
    messages: Arc<Mutex<Vec<String>>>,
}

impl BridgeServer {
    fn new(port: u16) -> Self {
        Self {
            port,
            peer: Arc::new(Mutex::new(None)),
            ws_tx: Arc::new(Mutex::new(None)),
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Connect to bpmux's WebSocket, identify, and forward messages until disconnected.
/// Returns when the connection drops.
async fn run_ws_connection(
    port: u16,
    peer: Peer<RoleServer>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
    messages_store: Arc<Mutex<Vec<String>>>,
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

    // Identify with bpmux
    {
        let mut tx = ws_tx_store.lock().await;
        if let Some(ref mut tx) = *tx {
            let identify = json!({"type": "identify", "name": "claude-code"}).to_string();
            if let Err(e) = tx.send(tungstenite::Message::Text(identify.into())).await {
                eprintln!("Bridge: failed to identify: {e}");
                return;
            }
        }
    }

    // Read welcome
    if let Some(Ok(_welcome)) = ws_rx.next().await {
        eprintln!("Bridge: connected to bpmux on port {port}");
    }

    // Forward bpmux messages → channel notifications until disconnected
    while let Some(Ok(frame)) = ws_rx.next().await {
        if let tungstenite::Message::Text(text) = frame {
            let msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            if let Ok(ServerMessage::Message { from, text, ts }) = msg {
                // Don't forward our own messages back
                if from.contains("claude-code") {
                    continue;
                }

                // Buffer the message for read_messages
                messages_store.lock().await.push(format!("[{from}] {text}"));

                let notification = CustomNotification::new(
                    "notifications/claude/channel",
                    Some(json!({
                        "content": format!("[{from}] {text}"),
                        "meta": {
                            "from": from,
                            "ts": ts,
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

    // Connection dropped — clear the sender so tool calls know we're disconnected
    *ws_tx_store.lock().await = None;
    eprintln!("Bridge: disconnected from bpmux");
}

impl ServerHandler for BridgeServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::builder().enable_tools().build();

        // Declare channel capability
        let mut experimental = std::collections::BTreeMap::new();
        experimental.insert("claude/channel".to_string(), serde_json::Map::new());
        capabilities.experimental = Some(experimental);

        ServerInfo::new(capabilities)
            .with_instructions("bpmux channel bridge — connects Claude Code to a bpmux session")
    }

    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let peer = context.peer.clone();
        let port = self.port;
        let peer_store = self.peer.clone();
        let ws_tx_store = self.ws_tx.clone();
        let messages_store = self.messages.clone();

        async move {
            *peer_store.lock().await = Some(peer.clone());

            // Reconnection loop — keeps trying to connect to bpmux
            tokio::spawn(async move {
                loop {
                    run_ws_connection(
                        port,
                        peer.clone(),
                        ws_tx_store.clone(),
                        messages_store.clone(),
                    )
                    .await;

                    // Wait before reconnecting
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
        let send_schema = json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The message text to send to the bpmux session"
                }
            },
            "required": ["text"]
        });

        let read_schema = json!({
            "type": "object",
            "properties": {}
        });

        let tools = vec![
            Tool::new(
                "send_message",
                "Send a message to the bpmux session. Your message will appear in the shared chat.",
                send_schema.as_object().cloned().unwrap_or_default(),
            ),
            Tool::new(
                "read_messages",
                "Read recent messages from the bpmux session.",
                read_schema.as_object().cloned().unwrap_or_default(),
            ),
        ];

        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let ws_tx = self.ws_tx.clone();
        let messages = self.messages.clone();

        async move {
            match request.name.as_ref() {
                "send_message" => {
                    let args = request.arguments.unwrap_or_default();
                    let text = match args.get("text").and_then(|v| v.as_str()) {
                        Some(t) => t,
                        None => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                "missing 'text' argument",
                            )]));
                        }
                    };

                    let send_msg = serde_json::to_string(&ClientMessage::Send {
                        text: text.to_string(),
                    })
                    .unwrap_or_default();

                    let mut tx = ws_tx.lock().await;
                    if let Some(ref mut tx) = *tx {
                        if let Err(e) = tx.send(tungstenite::Message::Text(send_msg.into())).await {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "failed to send: {e}"
                            ))]));
                        }
                    } else {
                        return Ok(CallToolResult::error(vec![Content::text(
                            "not connected to bpmux — reconnecting",
                        )]));
                    }

                    Ok(CallToolResult::success(vec![Content::text("message sent")]))
                }
                "read_messages" => {
                    let msgs = messages.lock().await;
                    let text = if msgs.is_empty() {
                        "No messages yet.".to_string()
                    } else {
                        msgs.join("\n")
                    };
                    Ok(CallToolResult::success(vec![Content::text(text)]))
                }
                other => Ok(CallToolResult::error(vec![Content::text(format!(
                    "unknown tool: {other}"
                ))])),
            }
        }
    }
}

pub fn run_bridge(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let server = BridgeServer::new(port);
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let service = server.serve((stdin, stdout)).await?;
        service.waiting().await?;
        anyhow::Ok(())
    })
}
