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

pub(crate) mod tools;
mod tools_stories;
pub(crate) mod tracker;

/// Public re-export so the transport layer can fire the same board-
/// invalidate marker that MCP bridge tools use (#313). Keeping one
/// constant means the TUI event handler stays a single branch — no
/// separate matching paths for MCP-originated vs REST-originated
/// invalidates.
pub use tools_stories::BOARD_INVALIDATE_MARKER;
pub use tools_stories::CI_STATE_UPDATE_MARKER;
pub use tools_stories::PINS_UPDATE_MARKER;
pub use tools_stories::REACTIONS_UPDATE_MARKER;
pub use tools_stories::ROSTER_UPDATE_MARKER;

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
                "Returns the current CI conclusion for bpmux main branch. Queries `gh run list --branch main -L 1` and returns a JSON object: { conclusion: 'success'|'failure'|'in_progress'|'unknown', last_run_id, branch: 'main', url, head_sha, display_title }. Pull-based — agents call this on-demand to make CI-aware decisions (ship gating, handoff signaling, pickup timing) without asking Ryan to relay. No permission gates; any role can call.",
            ),
            crate::mcp::tools::get_story_tool(
                "Fetch full detail for a single story by issue number. Returns JSON `{number, title, body, state, labels[]}`. Any role can call. USE THIS INSTEAD OF `gh issue view <N>` — raw gh calls bypass the shared chat so teammates can't observe the read (#403 closes that bypass). Required read before pickup-gate paraphrasing a story.",
            ),
            crate::mcp::tools::view_comments_tool(
                "Fetch the full comment audit-trail for a story by issue number. Returns JSON array `[{id, author, body, created_at}, ...]` in chronological order. Any role can call. USE THIS INSTEAD OF `gh issue view <N> --json comments` — surfaces rejection-reason + filing-discussion + retro-scenarios for rework-pickup, desk-check context, and audit-trail re-audit without reaching outside bpmux.",
            ),
            crate::mcp::tools::list_ready_tool(
                "Enumerate the full ready queue. Returns JSON array of `{number, title, labels[]}` for every story labeled `ready`. Any role can call. USE THIS INSTEAD OF `gh issue list --label ready` — view_backlog surfaces counts + a sample, list_ready returns the complete queue for full pickup-gate visibility.",
            ),
            // Story management tools — server-side gh proxy with lane enforcement
            crate::mcp::tools::view_board_tool(
                "View the project board — all stories grouped by state (ready, in progress, delivered, accepted, deferred, icebox) with counts. No host permission needed. Same data as the TUI board view.",
            ),
            crate::mcp::tools::view_velocity_tool(
                "View team velocity metrics (#209a) computed from workflow-signal timestamps over a rolling 14-day window. Returns JSON with delivered count, delivered-per-day, median pickup→deliver hours, median deliver→accept hours, rework rate, and a `sparse` flag when under-3-deliveries makes the medians indicative only. Objective data — no self-report. Also opens the velocity view in Ryan's TUI (project panel) so he can see it alongside your response — call this when he asks about velocity, not proactively.",
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
            crate::mcp::tools::close_iteration_tool(
                "Broadcast the iteration-close externalize-prompt (#282) so every agent flushes unsaved findings (via Write / save_lesson / add_principle / doc edit) before context compaction can evict them. PM ONLY — other roles are rejected. Fail-soft: tool returns after broadcast; iteration closes regardless of agent response.",
            ),
            crate::mcp::tools::label_story_tool(
                "Add a label to a story (ready, blocked, icebox, deferred). PM ONLY — other roles are rejected.",
            ),
            crate::mcp::tools::close_story_tool(
                "Close a story with a reason (superseded / duplicate / won't-fix). Adds the reason as a comment on the issue + closes it. PM ONLY — accept/reject are separate paths for delivered stories. Use this for pre-delivery closures that don't route through the ship chain. Args: number + reason (required).",
            ),
            crate::mcp::tools::comment_on_story_tool(
                "Add a comment to a story. Durable issue-audit-trail — visible to any future reader of the issue. Distinct from bpmux chat (session-scoped). Use for post-filing context (reconsider-notes, cross-refs, pickup-gate observations) you want persistent. Any agent role. Args: number + body (required; markdown supported).",
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
            crate::mcp::tools::view_roadmap_tool(
                "#226: surface the roadmap in Ryan's TUI. Call when conversation warrants reviewing roadmap state (e.g., 'what's on the roadmap?', 'what are we working on next?'). Same content as the /roadmap slash command; agents use this MCP tool instead of typing.",
            ),
            crate::mcp::tools::view_backlog_tool(
                "#286: surface the board/backlog in Ryan's TUI. Call when conversation warrants reviewing the backlog state (e.g., 'how's the icebox?', 'what's in the ready queue?', 'where are we?'). Returns a JSON summary with counts + ready/delivered samples so the agent can answer inline AND opens the project panel for Ryan.",
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
                    tools::handle_send(&args, &current_name, &ws_tx, &last_channel).await
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
                "get_ci_state" => tools::handle_get_ci_state(&args, &tracker).await,
                "get_story" => tools::handle_get_story(&args, &tracker).await,
                "view_comments" => tools::handle_view_comments(&args, &tracker).await,
                "list_ready" => tools::handle_list_ready(&tracker).await,
                "view_board" => tools_stories::handle_view_board(&args, &tracker).await,
                "view_velocity" => tools::handle_view_velocity(port, &tracker, &ws_tx).await,
                "create_story" => tools_stories::handle_create_story(&args, &ws_tx, &tracker).await,
                "start_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_start_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "deliver_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_deliver_story(&args, &current_name, &ws_tx, &tracker)
                        .await
                }
                "close_iteration" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_close_iteration(&current_name, &ws_tx).await
                }
                "label_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_label_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "close_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_close_story(&args, &current_name, &ws_tx, &tracker).await
                }
                "comment_on_story" => {
                    let current_name = name.lock().await;
                    tools_stories::handle_comment_on_story(&args, &current_name, &ws_tx, &tracker)
                        .await
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
                    tools_stories::handle_reorder_story(&args, &current_name, &ws_tx).await
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
                "view_tokens" => tools::handle_view_tokens(&ws_tx).await,
                "view_roadmap" => tools::handle_view_roadmap(&ws_tx).await,
                "view_backlog" => tools::handle_view_backlog(&ws_tx, &tracker).await,
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
        return rt.block_on(async {
            let server = BridgeServer::new(port, name.to_string(), tracker);
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let service = server.serve((stdin, stdout)).await?;
            service.waiting().await?;
            anyhow::Ok(())
        });
    }
    rt.block_on(async {
        let tracker = tracker::OctocrabStoryTracker::from_env().ok_or_else(|| {
            anyhow::anyhow!(
                "bpmux bridge requires GH_TOKEN (or GITHUB_TOKEN) + BPMUX_REPO \
                 env vars. Set both and retry. See diag log for which var was missing."
            )
        })?;
        let server = BridgeServer::new(port, name.to_string(), tracker);
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let service = server.serve((stdin, stdout)).await?;
        service.waiting().await?;
        anyhow::Ok(())
    })
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tools_stories_tests;
#[cfg(test)]
mod tracker_contract_tests;
#[cfg(test)]
mod tracker_tests;
