//! Bridge tool handlers — one module per MCP tool.

mod helpers;

mod add_principle;
#[cfg(test)]
mod add_stories_to_iteration_tests;
mod add_value;
mod capture_electron_window;
mod capture_screen;
mod classifier_misfire;
#[cfg(test)]
mod close_iteration_tests;
#[cfg(test)]
mod delete_iteration_marker_tests;
mod dev_in_flight;
mod get_bookmark;
mod get_briefing;
mod get_ci_log_for_run;
mod get_ci_state;
#[cfg(test)]
mod get_ci_state_tests;
mod get_identity;
mod get_story;
#[cfg(test)]
mod get_story_tests;
mod get_sync_status;
#[cfg(test)]
mod get_sync_status_tests;
mod get_token_report;
mod get_usage;
mod git_repo;
#[cfg(test)]
mod git_repo_tests;
mod iteration_markers;
mod list_epic_children;
#[cfg(test)]
mod list_epic_children_tests;
mod list_lessons;
mod list_peer_lessons;
mod list_ready;
#[cfg(test)]
mod list_ready_tests;
mod read_messages;
mod remove_principle;
#[cfg(test)]
mod remove_stories_from_iteration_tests;
#[cfg(test)]
mod reorder_iteration_marker_tests;
mod rerun_ci;
mod rework_context;
mod save_bookmark;
mod save_briefing;
mod save_lesson;
mod search_log;
mod send_message;
mod set_name;
mod set_status;
mod share_message;
mod simulate_click;
mod simulate_input_clear;
mod simulate_input_submit;
mod simulate_key;
mod simulate_type;
mod update_principle;
mod user_fn;
mod view_backlog;
mod view_comments;
mod view_commit_diff;
#[cfg(test)]
mod view_commit_diff_tests;
mod view_roadmap;
mod view_story_commits;
#[cfg(test)]
mod view_story_commits_tests;
mod view_story_timeline;
#[cfg(test)]
mod view_story_timeline_tests;
mod view_tokens;
mod view_velocity;

// Shared helpers re-exported for tools_stories.rs (sibling module
// inside bridge) which needs WsSink + ws_send.
pub(in crate::bridge) use helpers::{WsSink, ws_send};

pub(in crate::bridge) use add_principle::handle_add_principle;
pub(in crate::bridge) use add_value::handle_add_value;
pub(in crate::bridge) use capture_electron_window::handle_capture_electron_window;
pub(in crate::bridge) use capture_screen::handle_capture_screen;
pub(in crate::bridge) use classifier_misfire::handle_classifier_misfire;
pub(in crate::bridge) use dev_in_flight::handle_dev_in_flight;
pub(in crate::bridge) use get_bookmark::handle_get_bookmark;
pub(in crate::bridge) use get_briefing::handle_get_briefing;
pub(crate) use get_ci_log_for_run::CiLogError;
pub(crate) use get_ci_log_for_run::CiRunFailures;
pub(crate) use get_ci_log_for_run::CiTestFailure;
pub(in crate::bridge) use get_ci_log_for_run::handle_get_ci_log_for_run;
pub(crate) use get_ci_log_for_run::parse_playwright_failures_from_log;
pub(crate) use get_ci_state::CiStateReport;
pub(crate) use get_ci_state::build_ci_state_report;
pub(in crate::bridge) use get_ci_state::handle_get_ci_state;
pub(crate) use get_ci_state::unknown_ci_state_report;
pub(in crate::bridge) use get_identity::handle_get_identity;
pub(in crate::bridge) use get_story::handle_get_story;
pub(in crate::bridge) use get_sync_status::handle_get_sync_status;
pub(in crate::bridge) use get_token_report::handle_get_token_report;
pub(in crate::bridge) use get_usage::handle_get_usage;
#[cfg(test)]
pub(in crate::bridge) use iteration_markers::apply_reorder_iteration_marker;
pub(crate) use iteration_markers::{
    apply_add_stories_to_iteration, apply_delete_iteration_marker,
    apply_remove_stories_from_iteration,
};
pub(in crate::bridge) use iteration_markers::{
    handle_add_stories_to_iteration, handle_create_iteration_marker,
    handle_delete_iteration_marker, handle_list_iteration_markers,
    handle_remove_stories_from_iteration, handle_reorder_iteration_marker,
};
pub(in crate::bridge) use list_epic_children::handle_list_epic_children;
pub(in crate::bridge) use list_lessons::handle_list_lessons;
pub(in crate::bridge) use list_peer_lessons::handle_list_peer_lessons;
pub(in crate::bridge) use list_ready::handle_list_ready;
pub(in crate::bridge) use read_messages::handle_read_messages;
pub(in crate::bridge) use remove_principle::handle_remove_principle;
pub(in crate::bridge) use rerun_ci::handle_rerun_ci;
pub(in crate::bridge) use save_bookmark::handle_save_bookmark;
pub(in crate::bridge) use save_briefing::handle_save_briefing;
pub(in crate::bridge) use save_lesson::handle_save_lesson;
pub(in crate::bridge) use search_log::handle_search_log;
pub(in crate::bridge) use send_message::handle_send;
pub(in crate::bridge) use set_name::handle_set_name;
pub(in crate::bridge) use set_status::handle_set_status;
pub(in crate::bridge) use share_message::handle_share_message;
pub(in crate::bridge) use simulate_click::handle_simulate_click;
pub(in crate::bridge) use simulate_input_clear::handle_simulate_input_clear;
pub(in crate::bridge) use simulate_input_submit::handle_simulate_input_submit;
pub(in crate::bridge) use simulate_key::handle_simulate_key;
pub(in crate::bridge) use simulate_type::handle_simulate_type;
pub(in crate::bridge) use update_principle::handle_update_principle;
pub(in crate::bridge) use user_fn::{handle_list_user_fns, handle_user_fn};
pub(in crate::bridge) use view_backlog::build_backlog_summary;
pub(in crate::bridge) use view_comments::handle_view_comments;
pub(in crate::bridge) use view_commit_diff::handle_view_commit_diff;
pub(in crate::bridge) use view_roadmap::handle_view_roadmap;
pub(in crate::bridge) use view_story_commits::handle_view_story_commits;
pub(in crate::bridge) use view_story_timeline::handle_view_story_timeline;
pub(in crate::bridge) use view_tokens::handle_view_tokens;
pub(in crate::bridge) use view_velocity::{fetch_messages_via_http, handle_view_velocity};

pub use view_backlog::VIEW_BACKLOG_MARKER;

// Re-exports used only by bridge/tests.rs.
#[cfg(test)]
pub(in crate::bridge) use read_messages::curate_message;
#[cfg(test)]
pub(in crate::bridge) use share_message::{
    format_share_marker, parse_share_scope, pick_most_recent_visible_chat,
};
