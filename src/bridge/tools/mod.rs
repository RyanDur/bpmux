//! Bridge tool handlers — one module per MCP tool.

mod helpers;

mod add_principle;
mod add_value;
mod capture_screen;
mod classifier_misfire;
mod close_thread;
mod get_bookmark;
mod get_briefing;
mod get_ci_state;
mod get_identity;
mod get_token_report;
mod get_usage;
mod join_thread;
mod list_lessons;
mod list_peer_lessons;
mod pull_into_thread;
mod read_messages;
mod remove_principle;
mod save_bookmark;
mod save_briefing;
mod save_lesson;
mod search_log;
mod send_message;
mod set_name;
mod set_status;
mod share_message;
mod simulate_click;
mod simulate_key;
mod start_thread;
mod suggest_leave;
mod update_principle;
mod view_backlog;
mod view_roadmap;
mod view_tokens;
mod view_velocity;

// Shared helpers re-exported for tools_stories.rs (sibling module
// inside bridge) which needs WsSink + ws_send.
pub(in crate::bridge) use helpers::{WsSink, ws_send};

pub(in crate::bridge) use add_principle::handle_add_principle;
pub(in crate::bridge) use add_value::handle_add_value;
pub(in crate::bridge) use capture_screen::handle_capture_screen;
pub(in crate::bridge) use classifier_misfire::handle_classifier_misfire;
pub(in crate::bridge) use close_thread::handle_close_thread;
pub(in crate::bridge) use get_bookmark::handle_get_bookmark;
pub(in crate::bridge) use get_briefing::handle_get_briefing;
pub(in crate::bridge) use get_ci_state::handle_get_ci_state;
pub(in crate::bridge) use get_identity::handle_get_identity;
pub(in crate::bridge) use get_token_report::handle_get_token_report;
pub(in crate::bridge) use get_usage::handle_get_usage;
pub(in crate::bridge) use join_thread::handle_join_thread;
pub(in crate::bridge) use list_lessons::handle_list_lessons;
pub(in crate::bridge) use list_peer_lessons::handle_list_peer_lessons;
pub(in crate::bridge) use pull_into_thread::handle_pull_into_thread;
pub(in crate::bridge) use read_messages::handle_read_messages;
pub(in crate::bridge) use remove_principle::handle_remove_principle;
pub(in crate::bridge) use save_bookmark::handle_save_bookmark;
pub(in crate::bridge) use save_briefing::handle_save_briefing;
pub(in crate::bridge) use save_lesson::handle_save_lesson;
pub(in crate::bridge) use search_log::handle_search_log;
pub(in crate::bridge) use send_message::handle_send;
pub(in crate::bridge) use set_name::handle_set_name;
pub(in crate::bridge) use set_status::handle_set_status;
pub(in crate::bridge) use share_message::handle_share_message;
pub(in crate::bridge) use simulate_click::handle_simulate_click;
pub(in crate::bridge) use simulate_key::handle_simulate_key;
pub(in crate::bridge) use start_thread::handle_start_thread;
pub(in crate::bridge) use suggest_leave::handle_suggest_leave;
pub(in crate::bridge) use update_principle::handle_update_principle;
pub(in crate::bridge) use view_backlog::handle_view_backlog;
pub(in crate::bridge) use view_roadmap::handle_view_roadmap;
pub(in crate::bridge) use view_tokens::handle_view_tokens;
pub(in crate::bridge) use view_velocity::handle_view_velocity;

// Markers — re-exported so crate::bridge::tools::VIEW_*, etc. paths
// remain available if any caller relies on them.
#[allow(unused_imports)]
pub(in crate::bridge) use close_thread::THREAD_CLOSE_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use join_thread::THREAD_JOIN_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use pull_into_thread::PULL_INTO_THREAD_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use start_thread::THREAD_CREATE_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use suggest_leave::SUGGEST_LEAVE_MARKER;
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
