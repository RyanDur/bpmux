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
mod tools_stories;
pub(crate) mod tracker;

use crate::transport::protocol::ServerMessage;

type McpError = ErrorData;
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tungstenite::Message,
>;

#[derive(Clone)]
pub struct BridgeServer<T: tracker::StoryTracker> {
    port: u16,
    name: Arc<Mutex<String>>,
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    ws_tx: Arc<Mutex<Option<WsSink>>>,
    last_channel: Arc<Mutex<Option<Vec<String>>>>,
    tracker: T,
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

impl<T: tracker::StoryTracker> BridgeServer<T> {
    fn new(port: u16, name: String, tracker: T) -> Self {
        Self {
            port,
            name: Arc::new(Mutex::new(resolve_bridge_name(&name))),
            peer: Arc::new(Mutex::new(None)),
            ws_tx: Arc::new(Mutex::new(None)),
            last_channel: Arc::new(Mutex::new(None)),
            tracker,
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
/// Core fields (from/ts/channel/source) are always present. The three optional
/// fields (addressee/topic/message_type) are only included when non-None.
pub(super) fn build_notification_meta(
    from: &str,
    ts: &str,
    channel: &str,
    addressee: Option<&str>,
    topic: Option<&str>,
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
    if let Some(t) = topic {
        meta.insert("topic".to_string(), json!(t));
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
            topic,
            multi_depth: _,
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

            let meta = build_notification_meta(
                &from,
                &ts,
                &channel,
                addressee.as_deref(),
                topic.as_deref(),
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

impl<T: tracker::StoryTracker> ServerHandler for BridgeServer<T> {
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
            crate::mcp::tools::classifier_misfire_tool(
                "Report a classifier nudge that fired on a message you judge is NOT a learning moment. The reason field is OPTIONAL — per the 'state observation, not conclusion' framing, simply marking a fire as wrong is valuable training data on its own. Use this whenever you see an experience-nudge in the chat that you reason internally is incorrect, so the team can measure classifier accuracy and tune it over time. See #159.",
            ),
            crate::mcp::tools::save_bookmark_tool(
                "Save a session bookmark — structured snapshot of your current state (task, decisions, open questions, next step). Survives compaction. Use before long operations or when you sense compaction is near.",
            ),
            crate::mcp::tools::get_bookmark_tool(
                "Read a session bookmark — the structured state snapshot saved by an agent. Use after compaction to resume context quickly.",
            ),
            crate::mcp::tools::get_identity_tool(
                "Get an agent's identity traits — the stable principles and values that persist across sessions. Returns JSON with trait names and descriptions. Omit 'role' to get your own identity; pass a role name to see another agent's.",
            ),
            crate::mcp::tools::get_token_report_tool(
                "Get the token-cost report from bpmux's shared token log (#179). Returns JSON with total tokens, row count, per-role breakdown, and the 10 most recent turns (newest first). The data comes from per-harness adapters reading their own LLM-call recordings — you're seeing objective cost data, not self-reported. Same underlying log the `/tokens` TUI view reads, so you see exactly what Ryan sees. Empty report if the log is empty or missing.",
            ),
            // #140: story management tools — server-side gh proxy with lane enforcement
            crate::mcp::tools::view_board_tool(
                "View the project board — all stories grouped by state (ready, in progress, delivered, accepted, deferred, icebox) with counts. No host permission needed. Same data as the TUI board view.",
            ),
            crate::mcp::tools::view_velocity_tool(
                "View team velocity metrics (#209a) computed from workflow-signal timestamps over a rolling 14-day window. Returns JSON with delivered count, delivered-per-day, median pickup→deliver hours, median deliver→accept hours, rework rate, and a `sparse` flag when under-3-deliveries makes the medians indicative only. Objective data — no self-report.",
            ),
            crate::mcp::tools::create_story_tool(
                "Create a new story (GitHub issue). No host permission needed. Story starts unlabeled in the icebox.",
            ),
            crate::mcp::tools::start_story_tool(
                "Mark a story as started. DEV ONLY — other roles are rejected with a clear message.",
            ),
            crate::mcp::tools::deliver_story_tool(
                "Mark a story as delivered. DEV ONLY — other roles are rejected.",
            ),
            crate::mcp::tools::label_story_tool(
                "Add a label to a story (ready, blocked, icebox, deferred). PM ONLY — other roles are rejected.",
            ),
            crate::mcp::tools::accept_story_tool(
                "Accept a delivered story and close it. OWNER (ryan) ONLY.",
            ),
            crate::mcp::tools::reject_story_tool(
                "Reject a delivered story with a reason. OWNER (ryan) ONLY.",
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
            crate::mcp::tools::request_restart_tool(
                "#220: Request a clean restart of the bpmux TUI to pick up a new binary. Fires a system signal Ryan sees as a clickable 'restart available' affordance in the status bar. Ryan confirms or ignores — no auto-restart. Use when you know a binary rebuild has happened and want to offer Ryan the chance to apply it without leaving his flow.",
            ),
            crate::mcp::tools::get_usage_tool(
                "Get feature usage statistics — invocation counts and last-used timestamps for all commands/features used this session. JSON format.",
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
        let tracker = self.tracker.clone();

        async move {
            let args = request.arguments.unwrap_or_default();
            let result = match request.name.as_ref() {
                "send_message" => tools::handle_send(&args, &ws_tx, &last_channel).await,
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
                "simulate_click" => tools::handle_simulate_click(&args, port).await,
                "simulate_key" => tools::handle_simulate_key(&args, port).await,
                "classifier_misfire" => tools::handle_classifier_misfire(&args, port, &name).await,
                "save_bookmark" => {
                    let current_name = name.lock().await;
                    tools::handle_save_bookmark(&args, port, &current_name).await
                }
                "get_bookmark" => {
                    let current_name = name.lock().await;
                    tools::handle_get_bookmark(&args, port, &current_name).await
                }
                "get_identity" => {
                    let current_name = name.lock().await;
                    tools::handle_get_identity(&args, &current_name).await
                }
                "get_token_report" => tools::handle_get_token_report().await,
                "view_board" => tools_stories::handle_view_board(&args, &tracker).await,
                "view_velocity" => tools::handle_view_velocity(port).await,
                "create_story" => tools_stories::handle_create_story(&args, &tracker).await,
                "start_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_start_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "deliver_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_deliver_story(&args, &current_name, &ws_tx, &tracker)
                        .await
                }
                "label_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_label_story(&args, &current_name, &tracker).await
                }
                "accept_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_accept_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "reject_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_reject_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "reorder_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_reorder_story(&args, &current_name).await
                }
                "pass_desk_check" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_pass_desk_check(&args, &current_name, &ws_tx, &tracker)
                        .await
                }
                "fail_desk_check" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_fail_desk_check(&args, &current_name, &ws_tx, &tracker)
                        .await
                }
                "get_usage" => tools::handle_get_usage(&args).await,
                "request_restart" => tools::handle_request_restart(&ws_tx, &name).await,
                "view_tokens" => tools::handle_view_tokens(&ws_tx).await,
                other => Err(format!("unknown tool: {other}")),
            };
            Ok(match result {
                Ok(text) => CallToolResult::success(vec![Content::text(text)]),
                Err(e) => CallToolResult::error(vec![Content::text(e)]),
            })
        }
    }
}

pub fn run_bridge(
    port: u16,
    name: &str,
    fake_tracker: bool,
    board_fixture: Option<String>,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    if fake_tracker {
        let tracker = tracker::FakeTracker { board_fixture };
        rt.block_on(async {
            let server = BridgeServer::new(port, name.to_string(), tracker);
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let service = server.serve((stdin, stdout)).await?;
            service.waiting().await?;
            anyhow::Ok(())
        })
    } else if let Some(tracker) = tracker::OctocrabStoryTracker::from_env() {
        // #221: prefer the octocrab-backed tracker when auth + repo
        // are configured. Eliminates gh CLI shellouts from the
        // write path; direct REST API is faster and has cleaner
        // error shapes. GhTracker remains the fallback below.
        rt.block_on(async {
            let server = BridgeServer::new(port, name.to_string(), tracker);
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let service = server.serve((stdin, stdout)).await?;
            service.waiting().await?;
            anyhow::Ok(())
        })
    } else {
        let tracker = tracker::GhTracker { board_fixture };
        rt.block_on(async {
            let server = BridgeServer::new(port, name.to_string(), tracker);
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let service = server.serve((stdin, stdout)).await?;
            service.waiting().await?;
            anyhow::Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tools_stories_tests;
