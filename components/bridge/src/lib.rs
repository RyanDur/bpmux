use std::sync::Arc;

use rmcp::{ErrorData, ServiceExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub mod iteration_close;
pub use product_management::atp_substantive_gate;
pub mod chat_relevance;
pub mod connection;
pub use product_management::deliver_story_body_diff_gate;
pub mod dispatch;
pub mod event_subscription;
mod mcp_dispatch;
pub mod mcp_handler;
pub mod move_story;
pub mod polling;
mod rpc_client;
pub mod tool_registry;
pub mod tools;
pub mod tools_stories;
pub mod view_model;

pub use tool_registry::{DISPATCHED_TOOL_NAMES, build_advertised_tools};

pub use connection::run_ws_connection;

pub use chat_relevance::message_type_reaches_llm_stream;

#[cfg(test)]
pub use polling::{
    IngestTicker, SyncStatusSource, run_ingest_poll_loop, select_stories_to_hydrate,
    select_stories_to_revalidate,
};
pub use polling::{
    hydration_interval_from_env, poll_interval_from_env, run_sync_status_heartbeat,
    sync_status_heartbeat_interval_from_env,
};
pub use polling::{run_hydration_loop_cancellable, run_ingest_poll_loop_cancellable};

pub use dispatch::{
    dispatch_accept_story, dispatch_add_story_to_iteration, dispatch_close_iteration,
    dispatch_close_pre_pickup_review, dispatch_close_story, dispatch_comment_on_story,
    dispatch_create_iteration, dispatch_create_outcome, dispatch_create_story,
    dispatch_deliver_story, dispatch_designate_current_iteration, dispatch_dev_in_flight,
    dispatch_dev_ok_iteration, dispatch_edit_outcome, dispatch_edit_story,
    dispatch_fail_desk_check, dispatch_fail_stage, dispatch_get_story, dispatch_label_story,
    dispatch_list_epic_children, dispatch_list_ready, dispatch_map_iteration_to_outcome,
    dispatch_move_iteration_member, dispatch_move_story, dispatch_pass_desk_check,
    dispatch_pass_stage, dispatch_reject_story, dispatch_remove_iteration_member,
    dispatch_rename_iteration, dispatch_reorder_iteration, dispatch_reorder_outcome_iteration,
    dispatch_reorder_story, dispatch_reprioritize_outcome, dispatch_retire_outcome,
    dispatch_set_parent_epic, dispatch_start_story, dispatch_submit_verdict,
    dispatch_unmap_outcome_iteration, dispatch_unset_parent_epic, dispatch_view_backlog,
    dispatch_view_board, dispatch_view_comments, dispatch_view_commit_diff,
    dispatch_view_story_commits, dispatch_view_story_timeline, dispatch_view_velocity,
    dispatch_view_verdicts,
};

pub mod test_fixtures;

pub use tools::VIEW_BACKLOG_MARKER;
pub use tools_stories::parse_acceptance_test_path;

type McpError = ErrorData;
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tungstenite::Message,
>;

#[derive(Clone)]
pub struct BridgeServer {
    port: u16,
    name: Arc<Mutex<String>>,
    user_id: Option<String>,
    ws_tx: Arc<Mutex<Option<WsSink>>>,
    subscriptions: event_subscription::SubscriptionRegistry,
}

pub fn resolve_bridge_name(raw: &str) -> String {
    let is_placeholder = raw.starts_with("${") && raw.ends_with('}');
    if raw.is_empty() || raw == "claude-code" || is_placeholder {
        let suffix: String = uuid::Uuid::new_v4().to_string()[..4].to_string();
        format!("agent-{suffix}")
    } else {
        raw.to_string()
    }
}

impl BridgeServer {
    fn new(port: u16, name: String, user_id: Option<String>) -> Self {
        let resolved = resolve_bridge_name(&name);
        let subscriptions = event_subscription::SubscriptionRegistry::with_persistence(&resolved);
        Self {
            port,
            name: Arc::new(Mutex::new(resolved)),
            user_id,
            ws_tx: Arc::new(Mutex::new(None)),
            subscriptions,
        }
    }
}

pub fn run_bridge(
    port: u16,
    name: &str,
    user_id: Option<String>,
    _fake_tracker: bool,
    _board_fixture: Option<String>,
) -> anyhow::Result<()> {
    let project_root = workflow::load_project_root_or_cwd();
    let workflow = workflow::load_workflow(&project_root).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
        )
    })?;
    workflow::install_workflow(workflow);
    let ct = tokio_util::sync::CancellationToken::new();
    let restart_signal = self_update::RestartSignal::new();
    let rt = tokio::runtime::Runtime::new()?;
    let service_result: anyhow::Result<()> = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            let server = BridgeServer::new(port, name.to_string(), user_id);
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let service = server.serve((stdin, stdout)).await?;
            tokio::select! {
                result = service.waiting() => {
                    result?;
                }
                _ = ct.cancelled() => {}
            }
            anyhow::Ok(())
        }
    });
    if restart_signal.was_fired() {
        eprintln!(
            "[bpmux bridge] binary changed on disk; exiting with code {} so Claude Code respawns the MCP server with refreshed tools",
            self_update::RESTART_EXIT_CODE
        );
        std::process::exit(self_update::RESTART_EXIT_CODE);
    }
    service_result
}

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod chat_relevance_gate_tests;
#[cfg(test)]
mod connection_tests;
#[cfg(test)]
mod poller_middleware_tests;
#[cfg(test)]
mod rpc_dispatch;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_registry_tests;
#[cfg(test)]
mod tools_stories_tests;
#[cfg(test)]
mod view_translation;

#[cfg(test)]
mod workflow_test_support;
