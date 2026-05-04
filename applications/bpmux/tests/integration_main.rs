//! #1136 — consolidated integration test binary. 101 prior `tests/*_test.rs`
//! binaries each statically linked libbpmux + 100+ deps; CI link-time grew
//! linearly with test count and a SIGBUS on `recall_observability_test`
//! showed disk/memory pressure passing thresholds. Pattern: each
//! `tests/integration_main_modules/<name>.rs` is a submodule referenced
//! via `#[path]` here, so all test functions run from this single binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

#[path = "integration_main_modules/accept_story_e2e.rs"]
mod accept_story_e2e;
#[path = "integration_main_modules/agent_side_assertions.rs"]
mod agent_side_assertions;
#[path = "integration_main_modules/board_column_parity_e2e.rs"]
mod board_column_parity_e2e;
#[path = "integration_main_modules/board_fixture_seed.rs"]
mod board_fixture_seed;
#[path = "integration_main_modules/board_view.rs"]
mod board_view;
#[path = "integration_main_modules/bookmark_e2e.rs"]
mod bookmark_e2e;
#[path = "integration_main_modules/bridge_acceptance.rs"]
mod bridge_acceptance;
#[path = "integration_main_modules/bridge_collaboration.rs"]
mod bridge_collaboration;
#[path = "integration_main_modules/bridge_identity.rs"]
mod bridge_identity;
#[path = "integration_main_modules/briefing_e2e.rs"]
mod briefing_e2e;
#[path = "integration_main_modules/ci_canonical.rs"]
mod ci_canonical;
#[path = "integration_main_modules/classifier_misfire_e2e.rs"]
mod classifier_misfire_e2e;
#[path = "integration_main_modules/close_iteration_e2e.rs"]
mod close_iteration_e2e;
#[path = "integration_main_modules/close_story_e2e.rs"]
mod close_story_e2e;
#[path = "integration_main_modules/comment_on_story_e2e.rs"]
mod comment_on_story_e2e;
#[path = "integration_main_modules/create_story_e2e.rs"]
mod create_story_e2e;
#[path = "integration_main_modules/cross_bridge_mutation_propagation.rs"]
mod cross_bridge_mutation_propagation;
#[path = "integration_main_modules/deliver_story_e2e.rs"]
mod deliver_story_e2e;
#[path = "integration_main_modules/dev_in_flight_e2e.rs"]
mod dev_in_flight_e2e;
#[path = "integration_main_modules/diag_routing.rs"]
mod diag_routing;
#[path = "integration_main_modules/edit_story.rs"]
mod edit_story;
#[path = "integration_main_modules/fail_desk_check_e2e.rs"]
mod fail_desk_check_e2e;
#[path = "integration_main_modules/generic_move_story.rs"]
mod generic_move_story;
#[path = "integration_main_modules/get_ci_state_e2e.rs"]
mod get_ci_state_e2e;
#[path = "integration_main_modules/get_identity_e2e.rs"]
mod get_identity_e2e;
#[path = "integration_main_modules/get_story_e2e.rs"]
mod get_story_e2e;
#[path = "integration_main_modules/get_sync_status_e2e.rs"]
mod get_sync_status_e2e;
#[path = "integration_main_modules/get_token_report_e2e.rs"]
mod get_token_report_e2e;
#[path = "integration_main_modules/get_usage_e2e.rs"]
mod get_usage_e2e;
#[path = "integration_main_modules/gh_fixture_contract.rs"]
mod gh_fixture_contract;
#[path = "integration_main_modules/identity_mutation_e2e.rs"]
mod identity_mutation_e2e;
#[path = "integration_main_modules/label_story_e2e.rs"]
mod label_story_e2e;
#[path = "integration_main_modules/list_lessons_e2e.rs"]
mod list_lessons_e2e;
#[path = "integration_main_modules/list_peer_lessons_e2e.rs"]
mod list_peer_lessons_e2e;
#[path = "integration_main_modules/list_ready_e2e.rs"]
mod list_ready_e2e;
#[path = "integration_main_modules/mcp_advertisement_parity.rs"]
mod mcp_advertisement_parity;
#[path = "integration_main_modules/multi_depth_payload.rs"]
mod multi_depth_payload;
#[path = "integration_main_modules/pass_desk_check_e2e.rs"]
mod pass_desk_check_e2e;
#[path = "integration_main_modules/peer_count.rs"]
mod peer_count;
#[path = "integration_main_modules/plugin_acceptance.rs"]
mod plugin_acceptance;
#[path = "integration_main_modules/pty_direct.rs"]
mod pty_direct;
#[path = "integration_main_modules/read_messages_e2e.rs"]
mod read_messages_e2e;
#[path = "integration_main_modules/recall_observability.rs"]
mod recall_observability;
#[path = "integration_main_modules/reject_story_e2e.rs"]
mod reject_story_e2e;
#[path = "integration_main_modules/reorder_story_e2e.rs"]
mod reorder_story_e2e;
#[path = "integration_main_modules/roster_rename_reap_e2e.rs"]
mod roster_rename_reap_e2e;
#[path = "integration_main_modules/rpc_canonical_store.rs"]
mod rpc_canonical_store;
#[path = "integration_main_modules/save_lesson_e2e.rs"]
mod save_lesson_e2e;
#[path = "integration_main_modules/search_log_e2e.rs"]
mod search_log_e2e;
#[path = "integration_main_modules/send_claude_response_hook.rs"]
mod send_claude_response_hook;
#[path = "integration_main_modules/send_message_e2e.rs"]
mod send_message_e2e;
#[path = "integration_main_modules/session_persistence.rs"]
mod session_persistence;
#[path = "integration_main_modules/set_name_e2e.rs"]
mod set_name_e2e;
#[path = "integration_main_modules/set_parent_epic_e2e.rs"]
mod set_parent_epic_e2e;
#[path = "integration_main_modules/set_status_e2e.rs"]
mod set_status_e2e;
#[path = "integration_main_modules/share_message.rs"]
mod share_message;
#[path = "integration_main_modules/share_message_e2e.rs"]
mod share_message_e2e;
#[path = "integration_main_modules/sidebar_click.rs"]
mod sidebar_click;
#[path = "integration_main_modules/start_story_e2e.rs"]
mod start_story_e2e;
#[path = "integration_main_modules/state_field_rederive.rs"]
mod state_field_rederive;
#[path = "integration_main_modules/story2_acceptance.rs"]
mod story2_acceptance;
#[path = "integration_main_modules/story3_acceptance.rs"]
mod story3_acceptance;
#[path = "integration_main_modules/story51_acceptance.rs"]
mod story51_acceptance;
#[path = "integration_main_modules/story52_acceptance.rs"]
mod story52_acceptance;
#[path = "integration_main_modules/story55_acceptance.rs"]
mod story55_acceptance;
#[path = "integration_main_modules/story58_acceptance.rs"]
mod story58_acceptance;
#[path = "integration_main_modules/story59_acceptance.rs"]
mod story59_acceptance;
#[path = "integration_main_modules/story60_acceptance.rs"]
mod story60_acceptance;
#[path = "integration_main_modules/story61_acceptance.rs"]
mod story61_acceptance;
#[path = "integration_main_modules/story63_acceptance.rs"]
mod story63_acceptance;
#[path = "integration_main_modules/story64_acceptance.rs"]
mod story64_acceptance;
#[path = "integration_main_modules/story65_acceptance.rs"]
mod story65_acceptance;
#[path = "integration_main_modules/story66_acceptance.rs"]
mod story66_acceptance;
#[path = "integration_main_modules/story68_acceptance.rs"]
mod story68_acceptance;
#[path = "integration_main_modules/story69_acceptance.rs"]
mod story69_acceptance;
#[path = "integration_main_modules/story71_acceptance.rs"]
mod story71_acceptance;
#[path = "integration_main_modules/story81_acceptance.rs"]
mod story81_acceptance;
#[path = "integration_main_modules/story83_acceptance.rs"]
mod story83_acceptance;
#[path = "integration_main_modules/story85_acceptance.rs"]
mod story85_acceptance;
#[path = "integration_main_modules/story86_acceptance.rs"]
mod story86_acceptance;
#[path = "integration_main_modules/story87_acceptance.rs"]
mod story87_acceptance;
#[path = "integration_main_modules/token_adapter.rs"]
mod token_adapter;
#[path = "integration_main_modules/tokens_view.rs"]
mod tokens_view;
#[path = "integration_main_modules/unset_parent_epic_e2e.rs"]
mod unset_parent_epic_e2e;
#[path = "integration_main_modules/view_backlog.rs"]
mod view_backlog;
#[path = "integration_main_modules/view_backlog_e2e.rs"]
mod view_backlog_e2e;
#[path = "integration_main_modules/view_board_e2e.rs"]
mod view_board_e2e;
#[path = "integration_main_modules/view_comments_e2e.rs"]
mod view_comments_e2e;
#[path = "integration_main_modules/view_commit_diff_e2e.rs"]
mod view_commit_diff_e2e;
#[path = "integration_main_modules/view_roadmap.rs"]
mod view_roadmap;
#[path = "integration_main_modules/view_roadmap_e2e.rs"]
mod view_roadmap_e2e;
#[path = "integration_main_modules/view_story_commits_e2e.rs"]
mod view_story_commits_e2e;
#[path = "integration_main_modules/view_story_timeline_e2e.rs"]
mod view_story_timeline_e2e;
#[path = "integration_main_modules/view_tokens.rs"]
mod view_tokens;
#[path = "integration_main_modules/view_tokens_e2e.rs"]
mod view_tokens_e2e;
#[path = "integration_main_modules/view_velocity_e2e.rs"]
mod view_velocity_e2e;
#[path = "integration_main_modules/wait_helpers.rs"]
mod wait_helpers;
