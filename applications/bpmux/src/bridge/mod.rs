use std::sync::Arc;

use rmcp::service::Peer;
use rmcp::{ErrorData, RoleServer, ServiceExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

pub(crate) mod atp_substantive_gate;
pub(crate) mod chat_relevance;
pub(crate) mod connection;
pub(crate) mod deliver_story_body_diff_gate;
pub(crate) mod dispatch;
pub(crate) mod event_subscription;
mod mcp_dispatch;
pub(crate) mod mcp_handler;
pub(crate) mod move_story;
mod notification_meta;
pub(crate) mod polling;
mod rpc_client;
pub(crate) mod tool_registry;
pub(crate) mod tools;
pub(crate) mod tools_stories;
pub(crate) mod tracker;
pub(crate) mod view_model;

pub use tool_registry::{DISPATCHED_TOOL_NAMES, build_advertised_tools};

pub(crate) use connection::run_ws_connection;

pub use notification_meta::message_type_reaches_llm_stream;
pub(crate) use notification_meta::{build_notification_meta, validate_meta};

pub(crate) use polling::{
    drain_interval_from_env, hydration_interval_from_env, poll_interval_from_env, run_drain_loop,
};
pub use polling::{run_hydration_loop_cancellable, run_ingest_poll_loop_cancellable};
#[cfg(test)]
pub(crate) use polling::{
    run_ingest_poll_loop, select_stories_to_hydrate, select_stories_to_revalidate,
};

pub(crate) use dispatch::{
    dispatch_test_accept_story, dispatch_test_add_story_to_iteration,
    dispatch_test_close_iteration, dispatch_test_close_pre_pickup_review,
    dispatch_test_close_story, dispatch_test_comment_on_story, dispatch_test_create_iteration,
    dispatch_test_create_outcome, dispatch_test_create_story, dispatch_test_deliver_story,
    dispatch_test_designate_current_iteration, dispatch_test_dev_in_flight,
    dispatch_test_dev_ok_iteration, dispatch_test_edit_story, dispatch_test_fail_desk_check,
    dispatch_test_get_story, dispatch_test_label_story, dispatch_test_list_epic_children,
    dispatch_test_list_ready, dispatch_test_map_iteration_to_outcome, dispatch_test_move_story,
    dispatch_test_pass_desk_check, dispatch_test_reject_story, dispatch_test_reorder_story,
    dispatch_test_reprioritize_outcome, dispatch_test_set_parent_epic, dispatch_test_start_story,
    dispatch_test_submit_verdict, dispatch_test_unset_parent_epic, dispatch_test_view_backlog,
    dispatch_test_view_board, dispatch_test_view_comments, dispatch_test_view_commit_diff,
    dispatch_test_view_story_commits, dispatch_test_view_story_timeline,
    dispatch_test_view_velocity, dispatch_test_view_verdicts,
};

#[cfg(test)]
pub(crate) mod test_fixtures;

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
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    ws_tx: Arc<Mutex<Option<WsSink>>>,
    subscriptions: event_subscription::SubscriptionRegistry,
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
        let resolved = resolve_bridge_name(&name);
        let subscriptions = event_subscription::SubscriptionRegistry::with_persistence(&resolved);
        Self {
            port,
            name: Arc::new(Mutex::new(resolved)),
            peer: Arc::new(Mutex::new(None)),
            ws_tx: Arc::new(Mutex::new(None)),
            subscriptions,
        }
    }
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
    let ct = tokio_util::sync::CancellationToken::new();
    let restart_signal = crate::transport::self_update::RestartSignal::new();
    let rt = tokio::runtime::Runtime::new()?;
    let service_result: anyhow::Result<()> = rt.block_on({
        let ct = ct.clone();
        let restart_signal = restart_signal.clone();
        async move {
            crate::transport::self_update::spawn_binary_watcher(ct.clone(), restart_signal);
            let server = BridgeServer::new(port, name.to_string());
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
            crate::transport::self_update::RESTART_EXIT_CODE
        );
        std::process::exit(crate::transport::self_update::RESTART_EXIT_CODE);
    }
    service_result
}

#[cfg(test)]
mod connection_tests;
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
