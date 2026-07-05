use super::outbound_state_label;
use crate::product_management::story::{BoardState, Stage};

#[test]
fn desk_check_placement_wins_over_the_states_own_dev_stage() {
    assert_eq!(
        outbound_state_label(
            &BoardState::InProgress(Stage::Dev.id()),
            Some(&Stage::DeskCheck.id())
        ),
        "desk-check",
        "the fine sub-section rides placement — the coarse state's own stage does not decide it"
    );
}

#[test]
fn absent_placement_is_the_entry_sub_section_over_the_states_own_desk_check_stage() {
    assert_eq!(
        outbound_state_label(&BoardState::InProgress(Stage::DeskCheck.id()), None),
        "in progress",
        "no placement entry means the entry sub-section (dev), independent of the coarse state"
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
