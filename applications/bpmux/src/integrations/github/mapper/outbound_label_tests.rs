use super::outbound_state_label;
use crate::product_management::story::{BoardState, Stage};

#[test]
fn the_fine_middle_stage_is_not_leaked_to_the_github_label() {
    assert_eq!(
        outbound_state_label(&BoardState::InProgress, Some(&Stage::DeskCheck.id())),
        "in progress",
        "de-leak: GitHub carries only the coarse position; the fine dev/desk-check stage stays local in placements.json, never on the label"
    );
}

#[test]
fn in_progress_is_coarse_regardless_of_placement() {
    assert_eq!(
        outbound_state_label(&BoardState::InProgress, None),
        "in progress",
        "the outbound label is coarse whether or not a fine stage exists locally"
    );
}

#[test]
fn coarse_states_pass_through_to_their_github_label_ignoring_placement() {
    assert_eq!(outbound_state_label(&BoardState::Ready, None), "ready");
    assert_eq!(
        outbound_state_label(&BoardState::Acceptance, Some(&Stage::DeskCheck.id())),
        "delivered"
    );
    assert_eq!(
        outbound_state_label(&BoardState::Accepted, None),
        "accepted"
    );
    assert_eq!(outbound_state_label(&BoardState::Retired, None), "retired");
}
