#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod acceptance_test_path_tests;
mod add_story_to_iteration_tests;
mod atp_validator_tests;
mod close_pre_pickup_review_tests;
mod designate_current_iteration_tests;
mod handler_unit_tests;
mod integration_drain_tests;
mod label_story_iteration_retire_tests;
mod lifecycle_handler_tests;
mod mutation_handler_tests;
mod notification_bridge_tests;
mod observation_falsifier_tests;
mod pickup_tests;
mod reopen_transition_tests;
mod reorder_icebox_tests;
mod role_tests;
mod signal_tests;
mod stale_workflow_labels_tests;
mod verdict_substrate_tests;
mod view_board_basic_tests;
mod view_board_epic_tests;

pub(super) fn test_ci_state_store_green() -> crate::transport::ci_state_store::CiStateStore {
    let store = crate::transport::ci_state_store::CiStateStore::new();
    store.write(crate::bridge::tools::CiStateReport {
        conclusion: "success",
        last_run_id: None,
        branch: "main",
        url: String::new(),
        head_sha: Some("test-fixture-sha".to_string()),
        display_title: None,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 0,
        backoff_cap_secs: 0,
        workflows: Vec::new(),
    });
    store
}
