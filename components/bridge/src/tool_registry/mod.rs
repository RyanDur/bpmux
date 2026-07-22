mod board_read;
mod continuity;
mod lessons;
mod messaging;
mod observability;
mod presence;
mod story_workflow;
mod verdict_and_views;

use board_read::board_read_tools;
use continuity::continuity_tools;
use lessons::lessons_tools;
use messaging::messaging_tools;
use observability::observability_tools;
use presence::presence_tools;
use story_workflow::story_workflow_tools;
use verdict_and_views::verdict_and_view_tools;

pub fn build_advertised_tools() -> Vec<rmcp::model::Tool> {
    let mut tools = messaging_tools();
    tools.extend(lessons_tools());
    tools.extend(presence_tools());
    tools.extend(continuity_tools());
    tools.extend(observability_tools());
    tools.extend(board_read_tools());
    tools.extend(story_workflow_tools());
    tools.extend(verdict_and_view_tools());
    tools
}

pub const DISPATCHED_TOOL_NAMES: &[&str] = &[
    "send_message",
    "share_message",
    "subscribe",
    "unsubscribe",
    "wait_for_event",
    "set_name",
    "set_status",
    "read_messages",
    "who",
    "search_log",
    "list_lessons",
    "list_peer_lessons",
    "save_lesson",
    "capture_electron_window",
    "classifier_misfire",
    "save_bookmark",
    "get_bookmark",
    "save_briefing",
    "get_briefing",
    "get_identity",
    "add_value",
    "add_principle",
    "update_principle",
    "remove_principle",
    "get_token_report",
    "get_ci_state",
    "get_ci_log_for_run",
    "ci_get_job_status",
    "rerun_ci",
    "user_fn",
    "list_user_fns",
    "get_sync_status",
    "view_story_commits",
    "view_commit_diff",
    "view_velocity",
    "start_story",
    "deliver_story",
    "pass_desk_check",
    "fail_desk_check",
    "pass_stage",
    "fail_stage",
    "accept_story",
    "reject_story",
    "close_story",
    "create_story",
    "create_iteration",
    "comment_on_story",
    "label_story",
    "move_story",
    "set_parent_epic",
    "unset_parent_epic",
    "add_story_to_iteration",
    "designate_current_iteration",
    "reorder_story",
    "edit_story",
    "view_board",
    "get_story",
    "list_ready",
    "list_epic_children",
    "dev_in_flight",
    "view_story_timeline",
    "view_comments",
    "view_backlog",
    "close_iteration",
    "get_usage",
    "view_tokens",
    "view_roadmap",
    "submit_verdict",
    "view_verdicts",
];
