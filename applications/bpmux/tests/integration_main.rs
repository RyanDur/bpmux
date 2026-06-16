//! #1136 — consolidated integration test binary. 101 prior `tests/*_test.rs`
//! binaries each statically linked libbpmux + 100+ deps; CI link-time grew
//! linearly with test count and a SIGBUS on `recall_observability_test`
//! showed disk/memory pressure passing thresholds. Pattern: each
//! `tests/integration_main_modules/<name>.rs` is a submodule referenced
//! via `#[path]` here, so all test functions run from this single binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

#[path = "integration_main_modules/accept_story_concurrent_fire_e2e.rs"]
mod accept_story_concurrent_fire_e2e;
#[path = "integration_main_modules/accept_story_e2e.rs"]
mod accept_story_e2e;
#[path = "integration_main_modules/accept_story_substrate_enforce_e2e.rs"]
mod accept_story_substrate_enforce_e2e;
#[path = "integration_main_modules/acceptance_reject_substrate_enforce_e2e.rs"]
mod acceptance_reject_substrate_enforce_e2e;
#[path = "integration_main_modules/add_story_to_iteration_e2e.rs"]
mod add_story_to_iteration_e2e;
#[path = "integration_main_modules/addressed_to_me_e2e.rs"]
mod addressed_to_me_e2e;
#[path = "integration_main_modules/agent_side_assertions.rs"]
mod agent_side_assertions;
#[path = "integration_main_modules/atp_pre_filing_audit_substrate_enforce_e2e.rs"]
mod atp_pre_filing_audit_substrate_enforce_e2e;
#[path = "integration_main_modules/board_column_parity_e2e.rs"]
mod board_column_parity_e2e;
#[path = "integration_main_modules/board_fixture_seed.rs"]
mod board_fixture_seed;
#[path = "integration_main_modules/bookmark_e2e.rs"]
mod bookmark_e2e;
#[path = "integration_main_modules/bpmux_arrangement_verbs_cli_parity_e2e.rs"]
mod bpmux_arrangement_verbs_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_board_read_cli_e2e.rs"]
mod bpmux_board_read_cli_e2e;
#[path = "integration_main_modules/bpmux_bookmark_cli_parity_e2e.rs"]
mod bpmux_bookmark_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_briefing_cli_parity_e2e.rs"]
mod bpmux_briefing_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_capture_cli_parity_e2e.rs"]
mod bpmux_capture_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_ci_cli_parity_e2e.rs"]
mod bpmux_ci_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_creation_verbs_cli_parity_e2e.rs"]
mod bpmux_creation_verbs_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_epic_verbs_cli_parity_e2e.rs"]
mod bpmux_epic_verbs_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_insight_reads_cli_parity_e2e.rs"]
mod bpmux_insight_reads_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_iteration_add_cli_parity_e2e.rs"]
mod bpmux_iteration_add_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_iteration_close_cli_parity_e2e.rs"]
mod bpmux_iteration_close_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_lesson_cli_parity_e2e.rs"]
mod bpmux_lesson_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_pass_cli_substrate_parity_e2e.rs"]
mod bpmux_pass_cli_substrate_parity_e2e;
#[path = "integration_main_modules/bpmux_principle_cli_parity_e2e.rs"]
mod bpmux_principle_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_project_reads_cli_parity_e2e.rs"]
mod bpmux_project_reads_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_reads_rpc_cli_parity_e2e.rs"]
mod bpmux_reads_rpc_cli_parity_e2e;
#[path = "integration_main_modules/bpmux_register_cli_e2e.rs"]
mod bpmux_register_cli_e2e;
#[path = "integration_main_modules/bpmux_send_read_cli_e2e.rs"]
mod bpmux_send_read_cli_e2e;
#[path = "integration_main_modules/bpmux_send_speech_guard_parity_e2e.rs"]
mod bpmux_send_speech_guard_parity_e2e;
#[path = "integration_main_modules/bpmux_signal_cli_e2e.rs"]
mod bpmux_signal_cli_e2e;
#[path = "integration_main_modules/bpmux_workflow_verbs_cli_parity_e2e.rs"]
mod bpmux_workflow_verbs_cli_parity_e2e;
#[path = "integration_main_modules/bridge_collaboration.rs"]
mod bridge_collaboration;
#[path = "integration_main_modules/bridge_reconnect_e2e.rs"]
mod bridge_reconnect_e2e;
#[path = "integration_main_modules/briefing_e2e.rs"]
mod briefing_e2e;
#[path = "integration_main_modules/canonical_store_mutation_parity_e2e.rs"]
mod canonical_store_mutation_parity_e2e;
#[path = "integration_main_modules/capture_electron_window_e2e.rs"]
mod capture_electron_window_e2e;
#[path = "integration_main_modules/chat_no_reconnect_replay_e2e.rs"]
mod chat_no_reconnect_replay_e2e;
#[path = "integration_main_modules/chat_relevance_filter_e2e.rs"]
mod chat_relevance_filter_e2e;
#[path = "integration_main_modules/ci_canonical.rs"]
mod ci_canonical;
#[path = "integration_main_modules/ci_state_changed_e2e.rs"]
mod ci_state_changed_e2e;
#[path = "integration_main_modules/classifier_misfire_e2e.rs"]
mod classifier_misfire_e2e;
#[path = "integration_main_modules/close_iteration_e2e.rs"]
mod close_iteration_e2e;
#[path = "integration_main_modules/close_story_e2e.rs"]
mod close_story_e2e;
#[path = "integration_main_modules/close_story_substrate_enforce_e2e.rs"]
mod close_story_substrate_enforce_e2e;
#[path = "integration_main_modules/comment_on_story_e2e.rs"]
mod comment_on_story_e2e;
#[path = "integration_main_modules/create_story_e2e.rs"]
mod create_story_e2e;
#[path = "integration_main_modules/cross_bridge_mutation_propagation.rs"]
mod cross_bridge_mutation_propagation;
#[path = "integration_main_modules/deliver_story_e2e.rs"]
mod deliver_story_e2e;
#[path = "integration_main_modules/deliver_story_substrate_enforce_e2e.rs"]
mod deliver_story_substrate_enforce_e2e;
#[path = "integration_main_modules/dev_in_flight_e2e.rs"]
mod dev_in_flight_e2e;
#[path = "integration_main_modules/diag_routing.rs"]
mod diag_routing;
#[path = "integration_main_modules/edit_story.rs"]
mod edit_story;
#[path = "integration_main_modules/event_subscription_board_invalidated_e2e.rs"]
mod event_subscription_board_invalidated_e2e;
#[path = "integration_main_modules/event_subscription_persist_restart_e2e.rs"]
mod event_subscription_persist_restart_e2e;
#[path = "integration_main_modules/event_subscription_provisioned_live_e2e.rs"]
mod event_subscription_provisioned_live_e2e;
#[path = "integration_main_modules/event_subscription_replay_e2e.rs"]
mod event_subscription_replay_e2e;
#[path = "integration_main_modules/fail_desk_check_e2e.rs"]
mod fail_desk_check_e2e;
#[path = "integration_main_modules/fail_desk_check_substrate_enforce_e2e.rs"]
mod fail_desk_check_substrate_enforce_e2e;
#[path = "integration_main_modules/generic_move_story.rs"]
mod generic_move_story;
#[path = "integration_main_modules/get_ci_log_for_run_e2e.rs"]
mod get_ci_log_for_run_e2e;
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
#[path = "integration_main_modules/hook_auto_fire_skills_e2e.rs"]
mod hook_auto_fire_skills_e2e;
#[path = "integration_main_modules/identity_mutation_e2e.rs"]
mod identity_mutation_e2e;
#[path = "integration_main_modules/iteration_label_parity_e2e.rs"]
mod iteration_label_parity_e2e;
#[path = "integration_main_modules/label_story_e2e.rs"]
mod label_story_e2e;
#[path = "integration_main_modules/label_story_iteration_retire_e2e.rs"]
mod label_story_iteration_retire_e2e;
#[path = "integration_main_modules/list_lessons_e2e.rs"]
mod list_lessons_e2e;
#[path = "integration_main_modules/list_peer_lessons_e2e.rs"]
mod list_peer_lessons_e2e;
#[path = "integration_main_modules/list_ready_e2e.rs"]
mod list_ready_e2e;
#[path = "integration_main_modules/mcp_advertisement_parity.rs"]
mod mcp_advertisement_parity;
#[path = "integration_main_modules/move_reorder_substrate_enforce_e2e.rs"]
mod move_reorder_substrate_enforce_e2e;
#[path = "integration_main_modules/move_story_e2e.rs"]
mod move_story_e2e;
#[path = "integration_main_modules/move_story_retired_class_e2e.rs"]
mod move_story_retired_class_e2e;
#[path = "integration_main_modules/pass_desk_check_acceptor_arg_e2e.rs"]
mod pass_desk_check_acceptor_arg_e2e;
#[path = "integration_main_modules/pass_desk_check_e2e.rs"]
mod pass_desk_check_e2e;
#[path = "integration_main_modules/pass_desk_check_substrate_enforce_e2e.rs"]
mod pass_desk_check_substrate_enforce_e2e;
#[path = "integration_main_modules/plugin_acceptance.rs"]
mod plugin_acceptance;
#[path = "integration_main_modules/precompact_save_bookmark_e2e.rs"]
mod precompact_save_bookmark_e2e;
#[path = "integration_main_modules/read_messages_e2e.rs"]
mod read_messages_e2e;
#[path = "integration_main_modules/recall_observability.rs"]
mod recall_observability;
#[path = "integration_main_modules/receive_broadcast_hook_e2e.rs"]
mod receive_broadcast_hook_e2e;
#[path = "integration_main_modules/reject_story_e2e.rs"]
mod reject_story_e2e;
#[path = "integration_main_modules/rename_route_delivery_e2e.rs"]
mod rename_route_delivery_e2e;
#[path = "integration_main_modules/reorder_story_e2e.rs"]
mod reorder_story_e2e;
#[path = "integration_main_modules/reorder_story_retire_e2e.rs"]
mod reorder_story_retire_e2e;
#[path = "integration_main_modules/room_spawn_e2e.rs"]
mod room_spawn_e2e;
#[path = "integration_main_modules/roster_rename_reap_e2e.rs"]
mod roster_rename_reap_e2e;
#[path = "integration_main_modules/rpc_canonical_store.rs"]
mod rpc_canonical_store;
#[path = "integration_main_modules/save_lesson_e2e.rs"]
mod save_lesson_e2e;
#[path = "integration_main_modules/search_log_e2e.rs"]
mod search_log_e2e;
#[path = "integration_main_modules/section_skeleton_substrate_enforce_e2e.rs"]
mod section_skeleton_substrate_enforce_e2e;
#[path = "integration_main_modules/self_update_e2e.rs"]
mod self_update_e2e;
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
#[path = "integration_main_modules/set_status_retire_e2e.rs"]
mod set_status_retire_e2e;
#[path = "integration_main_modules/share_message_e2e.rs"]
mod share_message_e2e;
#[path = "integration_main_modules/start_story_e2e.rs"]
mod start_story_e2e;
#[path = "integration_main_modules/start_story_substrate_enforce_e2e.rs"]
mod start_story_substrate_enforce_e2e;
#[path = "integration_main_modules/state_field_rederive.rs"]
mod state_field_rederive;
#[path = "integration_main_modules/story59_acceptance.rs"]
mod story59_acceptance;
#[path = "integration_main_modules/story87_acceptance.rs"]
mod story87_acceptance;
#[path = "integration_main_modules/story_state_transition_e2e.rs"]
mod story_state_transition_e2e;
#[path = "integration_main_modules/sync_status_changed_e2e.rs"]
mod sync_status_changed_e2e;
#[path = "integration_main_modules/terminal_bridge_provision_e2e.rs"]
mod terminal_bridge_provision_e2e;
#[path = "integration_main_modules/terminal_receive_affordances_e2e.rs"]
mod terminal_receive_affordances_e2e;
#[path = "integration_main_modules/test_seed_e2e.rs"]
mod test_seed_e2e;
#[path = "integration_main_modules/token_adapter.rs"]
mod token_adapter;
#[path = "integration_main_modules/unset_parent_epic_e2e.rs"]
mod unset_parent_epic_e2e;
#[path = "integration_main_modules/view_backlog_e2e.rs"]
mod view_backlog_e2e;
#[path = "integration_main_modules/view_board_e2e.rs"]
mod view_board_e2e;
#[path = "integration_main_modules/view_comments_e2e.rs"]
mod view_comments_e2e;
#[path = "integration_main_modules/view_commit_diff_e2e.rs"]
mod view_commit_diff_e2e;
#[path = "integration_main_modules/view_roadmap_e2e.rs"]
mod view_roadmap_e2e;
#[path = "integration_main_modules/view_story_commits_e2e.rs"]
mod view_story_commits_e2e;
#[path = "integration_main_modules/view_story_timeline_e2e.rs"]
mod view_story_timeline_e2e;
#[path = "integration_main_modules/view_tokens_e2e.rs"]
mod view_tokens_e2e;
#[path = "integration_main_modules/view_velocity_e2e.rs"]
mod view_velocity_e2e;
#[path = "integration_main_modules/wait_for_event_cli_e2e.rs"]
mod wait_for_event_cli_e2e;
#[path = "integration_main_modules/wait_helpers.rs"]
mod wait_helpers;
