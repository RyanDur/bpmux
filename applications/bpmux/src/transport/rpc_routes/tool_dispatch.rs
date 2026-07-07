use std::sync::Arc;

use serde_json::Value;

use super::validation::known_tools;
use crate::bridge;
use crate::session;
use crate::sync::RefreshDriver;

pub(super) async fn dispatch(
    refresh: &Arc<dyn RefreshDriver>,
    tool: &str,
    agent: &str,
    args: serde_json::Map<String, Value>,
    session: &Arc<session::Session>,
    dispatcher: &crate::store::Dispatch,
) -> Result<String, String> {
    let refresh_ref: Option<&Arc<dyn RefreshDriver>> = Some(refresh);
    match tool {
        "start_story" => bridge::dispatch_start_story(args, agent, refresh_ref, session).await,
        "deliver_story" => bridge::dispatch_deliver_story(args, agent, refresh_ref, session).await,
        "pass_desk_check" => {
            bridge::dispatch_pass_desk_check(args, agent, refresh_ref, session).await
        }
        "fail_desk_check" => {
            bridge::dispatch_fail_desk_check(args, agent, refresh_ref, session).await
        }
        "pass_stage" => bridge::dispatch_pass_stage(args, agent, refresh_ref, session).await,
        "fail_stage" => bridge::dispatch_fail_stage(args, agent, refresh_ref, session).await,
        "accept_story" => bridge::dispatch_accept_story(args, agent, refresh_ref, session).await,
        "reject_story" => bridge::dispatch_reject_story(args, agent, refresh_ref, session).await,
        "close_story" => bridge::dispatch_close_story(args, agent, refresh_ref).await,
        "create_story" => bridge::dispatch_create_story(args, agent, refresh_ref, dispatcher).await,
        "create_iteration" => {
            bridge::dispatch_create_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "create_outcome" => {
            bridge::dispatch_create_outcome(args, agent, refresh_ref, dispatcher).await
        }
        "close_iteration" => {
            bridge::dispatch_close_iteration(args, agent, refresh_ref, session, dispatcher).await
        }
        "comment_on_story" => bridge::dispatch_comment_on_story(args, agent, refresh_ref).await,
        "label_story" => bridge::dispatch_label_story(args, agent, refresh_ref).await,
        "set_parent_epic" => bridge::dispatch_set_parent_epic(args, agent, refresh_ref).await,
        "unset_parent_epic" => bridge::dispatch_unset_parent_epic(args, agent, refresh_ref).await,
        "add_story_to_iteration" => {
            bridge::dispatch_add_story_to_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "designate_current_iteration" => {
            bridge::dispatch_designate_current_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "close_pre_pickup_review" => {
            bridge::dispatch_close_pre_pickup_review(args, agent, refresh_ref, dispatcher).await
        }
        "dev_ok_iteration" => {
            bridge::dispatch_dev_ok_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "map_iteration_to_outcome" => {
            bridge::dispatch_map_iteration_to_outcome(args, agent, refresh_ref, dispatcher).await
        }
        "reprioritize_outcome" => {
            bridge::dispatch_reprioritize_outcome(args, agent, refresh_ref, dispatcher).await
        }
        "edit_outcome" => bridge::dispatch_edit_outcome(args, agent, refresh_ref, dispatcher).await,
        "retire_outcome" => {
            bridge::dispatch_retire_outcome(args, agent, refresh_ref, dispatcher).await
        }
        "remove_iteration_member" => {
            bridge::dispatch_remove_iteration_member(args, agent, refresh_ref, dispatcher).await
        }
        "move_iteration_member" => {
            bridge::dispatch_move_iteration_member(args, agent, refresh_ref, dispatcher).await
        }
        "rename_iteration" => {
            bridge::dispatch_rename_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "unmap_outcome_iteration" => {
            bridge::dispatch_unmap_outcome_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "reorder_outcome_iteration" => {
            bridge::dispatch_reorder_outcome_iteration(args, agent, refresh_ref, dispatcher).await
        }
        "reorder_story" => bridge::dispatch_reorder_story(args, agent, refresh_ref).await,
        "move_story" => bridge::dispatch_move_story(args, agent, refresh_ref, dispatcher).await,
        "edit_story" => bridge::dispatch_edit_story(args, agent, refresh_ref, dispatcher).await,
        "view_board" => bridge::dispatch_view_board(args, refresh_ref).await,
        "get_story" => bridge::dispatch_get_story(args, refresh_ref).await,
        "list_ready" => bridge::dispatch_list_ready(refresh_ref).await,
        "list_epic_children" => bridge::dispatch_list_epic_children(args, refresh_ref).await,
        "dev_in_flight" => bridge::dispatch_dev_in_flight(refresh_ref).await,
        "view_story_timeline" => bridge::dispatch_view_story_timeline(args, refresh_ref).await,
        "view_comments" => bridge::dispatch_view_comments(args, refresh_ref).await,
        "view_backlog" => bridge::dispatch_view_backlog(refresh_ref).await,
        "submit_verdict" => bridge::dispatch_submit_verdict(args, agent, session).await,
        "view_verdicts" => bridge::dispatch_view_verdicts(args, session, refresh_ref).await,
        "view_velocity" => {
            let messages = session
                .read_messages()
                .map_err(|e| format!("view_velocity: read messages: {e}"))?;
            bridge::dispatch_view_velocity(messages, refresh_ref).await
        }
        "view_story_commits" => bridge::dispatch_view_story_commits(args).await,
        "view_commit_diff" => bridge::dispatch_view_commit_diff(args).await,
        _ => Err(format!(
            "rpc: unknown tool '{tool}'; expected one of {:?}",
            known_tools()
        )),
    }
}
