//! Bridge tool handlers — one module per MCP tool.

mod helpers;

mod add_principle;
mod add_value;
mod capture_screen;
mod classifier_misfire;
mod dev_in_flight;
mod get_bookmark;
mod get_briefing;
mod get_ci_state;
mod get_identity;
mod get_story;
#[cfg(test)]
mod get_story_tests;
mod get_token_report;
mod get_usage;
mod list_lessons;
mod list_peer_lessons;
mod list_ready;
#[cfg(test)]
mod list_ready_tests;
mod read_messages;
mod remove_principle;
mod rework_context;
#[cfg(test)]
mod rework_context_tests;
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
pub(in crate::bridge) use capture_screen::handle_capture_screen;
pub(in crate::bridge) use classifier_misfire::handle_classifier_misfire;
pub(in crate::bridge) use dev_in_flight::handle_dev_in_flight;
pub(in crate::bridge) use get_bookmark::handle_get_bookmark;
pub(in crate::bridge) use get_briefing::handle_get_briefing;
pub(crate) use get_ci_state::build_ci_state_report;
pub(in crate::bridge) use get_ci_state::handle_get_ci_state;
pub(in crate::bridge) use get_identity::handle_get_identity;
pub(in crate::bridge) use get_story::handle_get_story;
pub(in crate::bridge) use get_token_report::handle_get_token_report;
pub(in crate::bridge) use get_usage::handle_get_usage;
pub(in crate::bridge) use list_lessons::handle_list_lessons;
pub(in crate::bridge) use list_peer_lessons::handle_list_peer_lessons;
pub(in crate::bridge) use list_ready::handle_list_ready;
pub(in crate::bridge) use read_messages::handle_read_messages;
pub(in crate::bridge) use remove_principle::handle_remove_principle;
pub(in crate::bridge) use rework_context::format_rejection_comment;
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
pub(in crate::bridge) use view_backlog::handle_view_backlog;
pub(in crate::bridge) use view_comments::handle_view_comments;
pub(in crate::bridge) use view_commit_diff::handle_view_commit_diff;
pub(in crate::bridge) use view_roadmap::handle_view_roadmap;
pub(in crate::bridge) use view_story_commits::handle_view_story_commits;
pub(in crate::bridge) use view_story_timeline::{
    ACCEPT_GATE, ACCEPTED_MARKER, DESK_CHECK_PASS_GATE, PASS_DESK_CHECK_MARKER,
    format_gate_transition_comment, handle_view_story_timeline,
};
pub(in crate::bridge) use view_tokens::handle_view_tokens;
pub(in crate::bridge) use view_velocity::handle_view_velocity;

// Markers — re-exported so crate::bridge::tools::VIEW_*, etc. paths
// remain available if any caller relies on them.
#[allow(unused_imports)]
pub(in crate::bridge) use view_backlog::VIEW_BACKLOG_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use view_roadmap::VIEW_ROADMAP_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use view_tokens::VIEW_TOKENS_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use view_velocity::VIEW_VELOCITY_MARKER;

// Re-exports used only by bridge/tests.rs.
#[cfg(test)]
pub(in crate::bridge) use read_messages::curate_message;
#[cfg(test)]
pub(in crate::bridge) use share_message::{
    format_share_marker, parse_share_scope, pick_most_recent_visible_chat,
};
