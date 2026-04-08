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

async fn ws_send(ws_tx: &Mutex<Option<WsSink>>, payload: String) -> Result<(), String> {
    let mut tx = ws_tx.lock().await;
    let tx = tx.as_mut().ok_or("not connected to bpmux")?;
    tx.send(tungstenite::Message::Text(payload.into()))
        .await
        .map_err(|e| format!("send failed: {e}"))
}

async fn handle_send(
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("missing 'text' argument")?;
    let to = args.get("to").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });
    let json = serde_json::to_string(&ClientMessage::Send {
        text: text.to_string(),
        to,
        message_type: None,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok("message sent".into())
}

async fn handle_set_name(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &Mutex<String>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let new_name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument")?
        .to_string();
    *name.lock().await = new_name.clone();
    let json = serde_json::json!({"type": "rename", "name": new_name}).to_string();
    ws_send(ws_tx, json).await?;
    Ok(format!("name set to {new_name}"))
}

async fn handle_set_status(
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or("missing 'status' argument")?;
    let json = serde_json::to_string(&ClientMessage::Send {
        text: status.to_string(),
        to: None,
        message_type: Some("activity".into()),
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok(format!("status set to {status}"))
}

async fn handle_read_messages(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let default_limit = crate::transport::rest::DEFAULT_MESSAGE_LIMIT as u64;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default_limit);
    let url = format!("http://127.0.0.1:{port}/api/messages?limit={limit}");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let msgs: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|_| body.clone())?;
    if msgs.is_empty() {
        return Ok("No messages.".into());
    }
    Ok(msgs
        .iter()
        .filter_map(|m| {
            let from = m["from"].as_str()?;
            let text = m["text"].as_str()?;
            Some(format!("[{from}] {text}"))
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

async fn handle_search_log(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("missing 'query' argument")?;
    let mut url = format!(
        "http://127.0.0.1:{port}/api/search?query={}",
        urlencoding(query)
    );
    if let Some(sender) = args.get("sender").and_then(|v| v.as_str()) {
        url.push_str(&format!("&sender={sender}"));
    }
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to search: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let results: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|_| body.clone())?;
    if results.is_empty() {
        return Ok(format!("No results for \"{query}\"."));
    }
    let mut text = format!("── Search: \"{query}\" ({} results) ──\n", results.len());
    for (i, result) in results.iter().enumerate() {
        text.push_str(&format!("\n[{}]\n", i + 1));
        if let Some(msgs) = result["messages"].as_array() {
            for msg in msgs {
                let from = msg["from"].as_str().unwrap_or("?");
                let sender = from.split(':').next().unwrap_or(from);
                let msg_text = msg["text"].as_str().unwrap_or("");
                text.push_str(&format!("  {sender}: {msg_text}\n"));
            }
        }
    }
    text.push_str("── End of results ──");
    Ok(text)
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '?' => "%3F".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

async fn run_ws_connection(
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Peer<RoleServer>,
    ws_tx_store: Arc<Mutex<Option<WsSink>>>,
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
        let tools = vec![
            crate::mcp::tools::send_message_tool(
                "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
            ),
            crate::mcp::tools::read_messages_tool("Read recent messages from the bpmux session."),
            crate::mcp::tools::search_log_tool(
                "Search the session log by keyword. Returns message clusters with context. Use to find team corrections, decisions, and discussion arcs.",
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

        async move {
            let args = request.arguments.unwrap_or_default();
            let result = match request.name.as_ref() {
                "send_message" => handle_send(&args, &ws_tx).await,
                "set_name" => handle_set_name(&args, &name, &ws_tx).await,
                "set_status" => handle_set_status(&args, &ws_tx).await,
                "read_messages" => handle_read_messages(&args, port).await,
                "search_log" => handle_search_log(&args, port).await,
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
