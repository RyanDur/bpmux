use std::future::Future;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ListToolsResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{RoleServer, ServerHandler};

use super::tool_registry::build_advertised_tools;
use super::{BridgeServer, McpError, rpc_client, run_ws_connection, tools, tools_stories};

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
                "ci_get_job_status" => tools::handle_ci_get_job_status(&args, port).await,
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
                | "add_story_to_iteration"
                | "designate_current_iteration"
                | "reorder_story"
                | "edit_story"
                | "view_board"
                | "get_story"
                | "list_ready"
                | "list_epic_children"
                | "dev_in_flight"
                | "view_story_timeline"
                | "view_comments"
                | "view_backlog"
                | "submit_verdict"
                | "view_verdicts" => {
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
