//! Bridge tool handlers — one module per MCP tool.

mod helpers;

mod capture_screen;
mod classifier_misfire;
mod get_bookmark;
mod get_identity;
mod get_token_report;
mod get_usage;
mod list_lessons;
mod list_peer_lessons;
mod read_messages;
mod request_restart;
mod save_bookmark;
mod save_lesson;
mod search_log;
mod send_message;
mod set_name;
mod set_status;
mod share_message;
mod simulate_click;
mod simulate_key;
mod view_roadmap;
mod view_tokens;
mod view_velocity;

// Shared helpers re-exported for tools_stories.rs (sibling module
// inside bridge) which needs WsSink + ws_send.
pub(in crate::bridge) use helpers::{WsSink, ws_send};

pub(in crate::bridge) use capture_screen::handle_capture_screen;
pub(in crate::bridge) use classifier_misfire::handle_classifier_misfire;
pub(in crate::bridge) use get_bookmark::handle_get_bookmark;
pub(in crate::bridge) use get_identity::handle_get_identity;
pub(in crate::bridge) use get_token_report::handle_get_token_report;
pub(in crate::bridge) use get_usage::handle_get_usage;
pub(in crate::bridge) use list_lessons::handle_list_lessons;
pub(in crate::bridge) use list_peer_lessons::handle_list_peer_lessons;
pub(in crate::bridge) use read_messages::handle_read_messages;
pub(in crate::bridge) use request_restart::handle_request_restart;
pub(in crate::bridge) use save_bookmark::handle_save_bookmark;
pub(in crate::bridge) use save_lesson::handle_save_lesson;
pub(in crate::bridge) use search_log::handle_search_log;
pub(in crate::bridge) use send_message::handle_send;
pub(in crate::bridge) use set_name::handle_set_name;
pub(in crate::bridge) use set_status::handle_set_status;
pub(in crate::bridge) use share_message::handle_share_message;
pub(in crate::bridge) use simulate_click::handle_simulate_click;
pub(in crate::bridge) use simulate_key::handle_simulate_key;
pub(in crate::bridge) use view_roadmap::handle_view_roadmap;
pub(in crate::bridge) use view_tokens::handle_view_tokens;
pub(in crate::bridge) use view_velocity::handle_view_velocity;

// Markers — re-exported so crate::bridge::tools::VIEW_*, etc. paths
// remain available if any caller relies on them.
#[allow(unused_imports)]
pub(in crate::bridge) use request_restart::RESTART_REQUEST_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use view_roadmap::VIEW_ROADMAP_MARKER;
#[allow(unused_imports)]
pub(in crate::bridge) use view_tokens::VIEW_TOKENS_MARKER;

// Re-exports used only by bridge/tests.rs.
#[cfg(test)]
pub(in crate::bridge) use read_messages::curate_message;
#[cfg(test)]
pub(in crate::bridge) use share_message::{
    format_share_marker, parse_share_scope, pick_most_recent_visible_chat,
};
