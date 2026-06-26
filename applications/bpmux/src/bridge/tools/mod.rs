mod helpers;

mod add_principle;
mod add_value;
#[cfg(test)]
mod board_event_fire_ordering_tests;
mod capture_electron_window;
mod ci_get_job_status;
#[cfg(test)]
mod ci_get_job_status_tests;
#[cfg(test)]
mod ci_log_for_run_parse;
mod ci_log_parse;
mod classifier_misfire;
#[cfg(test)]
mod close_iteration_tests;
#[cfg(test)]
mod create_iteration_tests;
#[cfg(test)]
mod create_outcome_tests;
mod dev_in_flight;
#[cfg(test)]
mod dev_in_flight_query;
#[cfg(test)]
mod edit_outcome_tests;
#[cfg(test)]
mod edit_story_tests;
mod get_bookmark;
mod get_briefing;
#[cfg(test)]
mod get_briefing_tests;
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
pub(crate) mod git_repo;
#[cfg(test)]
mod git_repo_tests;
#[cfg(test)]
mod iteration_reorg_tests;
mod list_epic_children;
#[cfg(test)]
mod list_epic_children_tests;
mod list_lessons;
mod list_peer_lessons;
mod list_ready;
#[cfg(test)]
mod outcome_mapping_tests;
mod read_messages;
#[cfg(test)]
mod read_messages_agent_scoping;
mod remove_principle;
#[cfg(test)]
mod reprioritize_outcome_tests;
mod rerun_ci;
#[cfg(test)]
mod rerun_ci_dispatch;
#[cfg(test)]
mod retire_outcome_tests;
pub(in crate::bridge) mod rework_context;
mod save_bookmark;
mod save_briefing;
mod save_lesson;
mod search_log;
mod send_message;
#[cfg(test)]
mod send_message_tests;
mod set_name;
mod set_status;
mod share_message;
#[cfg(test)]
mod share_message_tests;
#[cfg(test)]
mod start_story_validation_tests;
mod submit_verdict;
#[cfg(test)]
mod submit_verdict_tests;
mod update_principle;
mod user_fn;
#[cfg(test)]
mod user_fn_dispatch;
mod user_fn_parse;
mod view_backlog;
#[cfg(test)]
mod view_backlog_render;
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
#[cfg(test)]
mod view_velocity_render;
mod view_verdicts;
#[cfg(test)]
mod view_verdicts_render;

pub(in crate::bridge) use helpers::{WsSink, ws_send};

pub(in crate::bridge) use add_principle::handle_add_principle;
pub(in crate::bridge) use add_value::handle_add_value;
pub(in crate::bridge) use capture_electron_window::handle_capture_electron_window;
pub(crate) use ci_get_job_status::CiJobRecord;
pub(crate) use ci_get_job_status::CiJobStatusError;
pub(crate) use ci_get_job_status::build_ci_job_status_response;
pub(crate) use ci_get_job_status::compute_duration_ms;
pub(in crate::bridge) use ci_get_job_status::handle_ci_get_job_status;
pub(crate) use ci_log_parse::build_non_test_failure_record;
pub(crate) use ci_log_parse::parse_playwright_failures_from_log;
pub(in crate::bridge) use classifier_misfire::handle_classifier_misfire;
pub(in crate::bridge) use dev_in_flight::handle_dev_in_flight;
pub(in crate::bridge) use get_bookmark::handle_get_bookmark;
pub(in crate::bridge) use get_briefing::handle_get_briefing;
pub(crate) use get_ci_log_for_run::CiLogError;
pub(crate) use get_ci_log_for_run::CiRunFailures;
pub(crate) use get_ci_log_for_run::CiTestFailure;
pub(in crate::bridge) use get_ci_log_for_run::handle_get_ci_log_for_run;
pub(crate) use get_ci_state::CiStateReport;
pub(crate) use get_ci_state::build_ci_state_report;
pub(in crate::bridge) use get_ci_state::handle_get_ci_state;
pub(crate) use get_ci_state::unknown_ci_state_report;
pub(in crate::bridge) use get_identity::handle_get_identity;
pub(in crate::bridge) use get_story::handle_get_story;
pub(in crate::bridge) use get_sync_status::handle_get_sync_status;
pub(in crate::bridge) use get_token_report::handle_get_token_report;
pub(in crate::bridge) use get_usage::handle_get_usage;
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
pub(in crate::bridge) use submit_verdict::handle_submit_verdict;
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
pub(in crate::bridge) use view_verdicts::handle_view_verdicts;

pub use view_backlog::VIEW_BACKLOG_MARKER;

#[cfg(test)]
pub(in crate::bridge) use read_messages::curate_message;
#[cfg(test)]
pub(in crate::bridge) use share_message::{
    format_share_marker, parse_share_scope, pick_most_recent_visible_chat,
};
