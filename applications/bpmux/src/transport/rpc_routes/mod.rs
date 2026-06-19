use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde_json::Value;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;
use crate::session;

mod state_delta;
mod validation;

#[cfg(test)]
mod rpc_dispatch_tests;

use state_delta::{
    fire_board_invalidate, fire_board_snapshot, fire_story_transition, fire_view_backlog_marker,
    story_state_label,
};
pub use validation::{AGENT_HEADER, agent_from_headers};
use validation::{args_from_value, is_read_only_tool, known_tools};

async fn dispatch(
    refresh: &Arc<dyn RefreshDriver>,
    tool: &str,
    agent: &str,
    args: serde_json::Map<String, Value>,
    session: &Arc<session::Session>,
) -> Result<String, String> {
    let refresh_ref: Option<&Arc<dyn RefreshDriver>> = Some(refresh);
    match tool {
        "start_story" => bridge::dispatch_test_start_story(args, agent, refresh_ref, session).await,
        "deliver_story" => {
            bridge::dispatch_test_deliver_story(args, agent, refresh_ref, session).await
        }
        "pass_desk_check" => {
            bridge::dispatch_test_pass_desk_check(args, agent, refresh_ref, session).await
        }
        "fail_desk_check" => {
            bridge::dispatch_test_fail_desk_check(args, agent, refresh_ref, session).await
        }
        "accept_story" => {
            bridge::dispatch_test_accept_story(args, agent, refresh_ref, session).await
        }
        "reject_story" => {
            bridge::dispatch_test_reject_story(args, agent, refresh_ref, session).await
        }
        "close_story" => bridge::dispatch_test_close_story(args, agent, refresh_ref).await,
        "create_story" => bridge::dispatch_test_create_story(args, agent, refresh_ref).await,
        "create_iteration" => {
            bridge::dispatch_test_create_iteration(args, agent, refresh_ref).await
        }
        "create_outcome" => bridge::dispatch_test_create_outcome(args, agent, refresh_ref).await,
        "close_iteration" => {
            bridge::dispatch_test_close_iteration(args, agent, refresh_ref, session).await
        }
        "comment_on_story" => {
            bridge::dispatch_test_comment_on_story(args, agent, refresh_ref).await
        }
        "label_story" => bridge::dispatch_test_label_story(args, agent, refresh_ref).await,
        "set_parent_epic" => bridge::dispatch_test_set_parent_epic(args, agent, refresh_ref).await,
        "unset_parent_epic" => {
            bridge::dispatch_test_unset_parent_epic(args, agent, refresh_ref).await
        }
        "add_story_to_iteration" => {
            bridge::dispatch_test_add_story_to_iteration(args, agent, refresh_ref).await
        }
        "designate_current_iteration" => {
            bridge::dispatch_test_designate_current_iteration(args, agent, refresh_ref).await
        }
        "close_pre_pickup_review" => {
            bridge::dispatch_test_close_pre_pickup_review(args, agent, refresh_ref).await
        }
        "dev_ok_iteration" => {
            bridge::dispatch_test_dev_ok_iteration(args, agent, refresh_ref).await
        }
        "map_iteration_to_outcome" => {
            bridge::dispatch_test_map_iteration_to_outcome(args, agent, refresh_ref).await
        }
        "reorder_story" => bridge::dispatch_test_reorder_story(args, agent, refresh_ref).await,
        "move_story" => bridge::dispatch_test_move_story(args, agent, refresh_ref).await,
        "edit_story" => bridge::dispatch_test_edit_story(args, agent, refresh_ref).await,
        "view_board" => bridge::dispatch_test_view_board(args, refresh_ref).await,
        "get_story" => bridge::dispatch_test_get_story(args, refresh_ref).await,
        "list_ready" => bridge::dispatch_test_list_ready(refresh_ref).await,
        "list_epic_children" => bridge::dispatch_test_list_epic_children(args, refresh_ref).await,
        "dev_in_flight" => bridge::dispatch_test_dev_in_flight(refresh_ref).await,
        "view_story_timeline" => bridge::dispatch_test_view_story_timeline(args, refresh_ref).await,
        "view_comments" => bridge::dispatch_test_view_comments(args, refresh_ref).await,
        "view_backlog" => bridge::dispatch_test_view_backlog(refresh_ref).await,
        "submit_verdict" => bridge::dispatch_test_submit_verdict(args, agent, session).await,
        "view_verdicts" => bridge::dispatch_test_view_verdicts(args, session, refresh_ref).await,
        "view_velocity" => {
            let messages = session
                .read_messages()
                .map_err(|e| format!("view_velocity: read messages: {e}"))?;
            bridge::dispatch_test_view_velocity(messages, refresh_ref).await
        }
        "view_story_commits" => bridge::dispatch_test_view_story_commits(args).await,
        "view_commit_diff" => bridge::dispatch_test_view_commit_diff(args).await,
        _ => Err(format!(
            "rpc: unknown tool '{tool}'; expected one of {:?}",
            known_tools()
        )),
    }
}

pub async fn post_rpc_tool(
    State(session): State<Arc<session::Session>>,
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let agent = match agent_from_headers(&headers) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let agent = crate::workflow::current_workflow()
        .role_id_for_purpose(&agent)
        .map(str::to_string)
        .unwrap_or(agent);
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "canonical store not configured (no GitHub orchestrator built at startup)"
            })),
        );
    };
    let args = match args_from_value(body) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e })),
            );
        }
    };
    let transition_number = args.get("number").and_then(|v| v.as_u64());
    let declared_target_state = if tool == "move_story" {
        args.get("target_state")
            .and_then(|v| v.as_str())
            .and_then(crate::board::domain::StoryState::parse)
            .map(|s| s.as_str())
    } else {
        None
    };
    let prior_state = transition_number
        .filter(|_| !is_read_only_tool(&tool))
        .and_then(|number| story_state_label(&refresh, number));
    match dispatch(&refresh, &tool, &agent, args, &session).await {
        Ok(message) => {
            if tool == "view_backlog" {
                fire_view_backlog_marker(&session);
            } else if !is_read_only_tool(&tool) {
                fire_board_invalidate(&session);
                fire_board_snapshot(&session, &refresh);
                if let (Some(number), Some(from_state)) = (transition_number, prior_state)
                    && let Some(target_state) =
                        declared_target_state.or_else(|| story_state_label(&refresh, number))
                    && target_state != from_state
                {
                    fire_story_transition(&session, number, from_state, target_state, &agent);
                }
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({ "message": message })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}
