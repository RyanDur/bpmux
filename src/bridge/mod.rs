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

use crate::transport::protocol::{ClientMessage, ServerMessage};

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
    messages: Arc<Mutex<Vec<String>>>,
    last_read_index: Arc<Mutex<usize>>,
}

/// Resolve the bridge's display name. The CLI default "claude-code" is a
/// sentinel that triggers `agent-XXXX` auto-generation. Empty strings and
/// unexpanded env-var placeholders (e.g. "${BPMUX_ROLE}" when the var isn't
/// set) also fall through to auto-generation so the bridge never identifies
/// itself with a broken value.
fn resolve_bridge_name(raw: &str) -> String {
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
            messages: Arc::new(Mutex::new(Vec::new())),
            last_read_index: Arc::new(Mutex::new(0)),
        }
    }
}

/// Connect to bpmux's WebSocket, identify, and forward messages until disconnected.
/// Returns when the connection drops.
async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
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

    // Read welcome
    if let Some(Ok(_welcome)) = ws_rx.next().await {
        eprintln!("Bridge: connected to bpmux on port {port}");
    }

    // Backfill missed messages from REST API on reconnect
    let backfill_url = format!("http://127.0.0.1:{port}/api/messages");
    if let Ok(output) = std::process::Command::new("curl")
        .args(["-s", &backfill_url])
        .output()
        && let Ok(body) = String::from_utf8(output.stdout)
        && let Ok(msgs) = serde_json::from_str::<Vec<serde_json::Value>>(&body)
    {
        let mut store = messages_store.lock().await;
        store.clear();
        for msg in msgs {
            if let (Some(from), Some(text)) = (msg["from"].as_str(), msg["text"].as_str()) {
                store.push(format!("[{from}] {text}"));
            }
        }
        eprintln!("Bridge: backfilled {} messages", store.len());
    }

    // Forward bpmux messages → channel notifications until disconnected
    while let Some(Ok(frame)) = ws_rx.next().await {
        if let tungstenite::Message::Text(text) = frame {
            let msg: Result<ServerMessage, _> = serde_json::from_str(&text);
            if let Ok(ServerMessage::Message { from, text, ts, .. }) = msg {
                // Don't forward our own messages back
                let current_name = name.lock().await;
                if from.contains(current_name.as_str()) {
                    drop(current_name);
                    continue;
                }
                drop(current_name);

                // Buffer ALL messages for read_messages (pull model)
                messages_store.lock().await.push(format!("[{from}] {text}"));

                // Push every non-self message. Agents need to see each other
                // to collaborate; the @mention-only filter caused parallel
                // monologues. Let data drive any future filtering.
                let short_name = from.split(':').next().unwrap_or(&from);
                let notification = CustomNotification::new(
                    "notifications/claude/channel",
                    Some(json!({
                        "content": format!("{short_name}: {text}"),
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
        let messages_store = self.messages.clone();

        async move {
            *peer_store.lock().await = Some(peer.clone());

            // Reconnection loop — keeps trying to connect to bpmux
            tokio::spawn(async move {
                loop {
                    run_ws_connection(
                        port,
                        name.clone(),
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
        let tools = vec![
            crate::mcp::tools::send_message_tool(
                "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
            ),
            crate::mcp::tools::read_messages_tool("Read recent messages from the bpmux session."),
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
        let messages = self.messages.clone();
        let last_read_index = self.last_read_index.clone();

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

                    let to: Option<Vec<String>> =
                        args.get("to").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });

                    let send_msg = match serde_json::to_string(&ClientMessage::Send {
                        text: text.to_string(),
                        to,
                        message_type: None,
                    }) {
                        Ok(json) => json,
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "failed to serialize message: {e}"
                            ))]));
                        }
                    };

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
                "set_name" => {
                    let args = request.arguments.unwrap_or_default();
                    let new_name = match args.get("name").and_then(|v| v.as_str()) {
                        Some(n) => n.to_string(),
                        None => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                "missing 'name' argument",
                            )]));
                        }
                    };

                    // Update the bridge's internal name
                    *name.lock().await = new_name.clone();

                    // Send rename to bpmux server
                    let rename_msg =
                        serde_json::json!({"type": "rename", "name": new_name}).to_string();
                    let mut tx = ws_tx.lock().await;
                    if let Some(ref mut tx) = *tx
                        && let Err(e) = tx.send(tungstenite::Message::Text(rename_msg.into())).await
                    {
                        return Ok(CallToolResult::error(vec![Content::text(format!(
                            "failed to rename: {e}"
                        ))]));
                    }

                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "name set to {new_name}"
                    ))]))
                }
                "set_status" => {
                    let args = request.arguments.unwrap_or_default();
                    let status = match args.get("status").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                "missing 'status' argument",
                            )]));
                        }
                    };

                    // Send as activity message via REST API so TUI status bar picks it up
                    let current_name = name.lock().await.clone();
                    let api_url = format!("http://127.0.0.1:{port}/api/messages");
                    let body = json!({
                        "from": current_name,
                        "text": status,
                        "message_type": "activity"
                    })
                    .to_string();
                    let result = tokio::process::Command::new("curl")
                        .args([
                            "-s",
                            "-X",
                            "POST",
                            &api_url,
                            "-H",
                            "Content-Type: application/json",
                            "-d",
                            &body,
                        ])
                        .output()
                        .await;
                    if let Err(e) = result {
                        return Ok(CallToolResult::error(vec![Content::text(format!(
                            "failed to set status: {e}"
                        ))]));
                    }

                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "status set to {status}"
                    ))]))
                }
                "read_messages" => {
                    let args = request.arguments.unwrap_or_default();
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
                    let msgs = messages.lock().await;
                    let mut idx = last_read_index.lock().await;
                    let new_msgs: Vec<&String> = msgs.iter().skip(*idx).take(limit).collect();
                    *idx = (*idx + new_msgs.len()).min(msgs.len());
                    let text = if new_msgs.is_empty() {
                        "No new messages.".to_string()
                    } else {
                        new_msgs
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// Simulates the read_messages pagination logic extracted from call_tool.
    async fn read_with_limit(
        messages: &Mutex<Vec<String>>,
        last_read_index: &Mutex<usize>,
        limit: usize,
    ) -> String {
        let msgs = messages.lock().await;
        let mut idx = last_read_index.lock().await;
        let new_msgs: Vec<&String> = msgs.iter().skip(*idx).take(limit).collect();
        *idx = (*idx + new_msgs.len()).min(msgs.len());
        if new_msgs.is_empty() {
            "No new messages.".to_string()
        } else {
            new_msgs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    fn seed_messages(count: usize) -> (Mutex<Vec<String>>, Mutex<usize>) {
        let msgs: Vec<String> = (0..count).map(|i| format!("[user] msg {i}")).collect();
        (Mutex::new(msgs), Mutex::new(0))
    }

    #[test]
    fn resolve_bridge_name_uses_given_name_when_provided() {
        assert_eq!(resolve_bridge_name("dev"), "dev");
        assert_eq!(resolve_bridge_name("pm"), "pm");
        assert_eq!(resolve_bridge_name("designer"), "designer");
    }

    #[test]
    fn resolve_bridge_name_generates_agent_suffix_for_default() {
        let name = resolve_bridge_name("claude-code");
        assert!(
            name.starts_with("agent-"),
            "expected agent-XXXX, got: {name}"
        );
        assert_eq!(name.len(), "agent-".len() + 4);
    }

    #[test]
    fn resolve_bridge_name_generates_agent_suffix_when_empty() {
        let name = resolve_bridge_name("");
        assert!(
            name.starts_with("agent-"),
            "empty name should auto-generate, got: {name}"
        );
    }

    #[test]
    fn resolve_bridge_name_generates_agent_suffix_for_unexpanded_placeholder() {
        let name = resolve_bridge_name("${BPMUX_ROLE}");
        assert!(
            name.starts_with("agent-"),
            "unexpanded placeholder should auto-generate, got: {name}"
        );
    }

    #[tokio::test]
    async fn reads_messages_with_limit() {
        let (messages, idx) = seed_messages(5);
        let text = read_with_limit(&messages, &idx, 2).await;
        assert!(text.contains("msg 0"), "should contain first message");
        assert!(text.contains("msg 1"), "should contain second message");
        assert!(!text.contains("msg 2"), "should not contain third message");

        // Next read picks up from where we left off
        let text2 = read_with_limit(&messages, &idx, 2).await;
        assert!(text2.contains("msg 2"), "should continue from index 2");
        assert!(text2.contains("msg 3"), "should contain msg 3");
        assert!(!text2.contains("msg 4"), "should not contain msg 4");
    }

    #[tokio::test]
    async fn reads_messages_defaults_to_twenty() {
        let (messages, idx) = seed_messages(25);
        let text = read_with_limit(&messages, &idx, 20).await;
        assert!(text.contains("msg 19"), "should contain msg 19");
        assert!(!text.contains("msg 20"), "should not contain msg 20");
    }

    #[tokio::test]
    async fn parses_limit_from_tool_args() {
        let args = serde_json::json!({"limit": 5});
        let limit = args
            .as_object()
            .and_then(|m| m.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        assert_eq!(limit, 5);

        let empty_args = serde_json::json!({});
        let default_limit = empty_args
            .as_object()
            .and_then(|m| m.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        assert_eq!(default_limit, 20);
    }
}
