use super::outbound_state_label;
use crate::product_management::story::BoardState;

#[test]
fn the_outbound_label_is_the_coarse_position_never_a_fine_stage() {
    assert_eq!(
        outbound_state_label(&BoardState::InProgress, false),
        Some("in progress"),
        "de-leak: in-progress goes out coarse; the fine dev/desk-check stage stays local in placements.json, never on the label"
    );
    assert_eq!(
        outbound_state_label(&BoardState::Acceptance, false),
        Some("delivered")
    );
    assert_eq!(
        outbound_state_label(&BoardState::Accepted, false),
        Some("accepted")
    );
    assert_eq!(
        outbound_state_label(&BoardState::Retired, false),
        Some("retired")
    );
}

#[test]
fn a_not_started_current_iteration_member_derives_the_ready_mirror_label() {
    assert_eq!(
        outbound_state_label(&BoardState::NotStarted, true),
        Some("ready"),
        "the human-readable GitHub mirror shows ready for scheduled not-started work — derived from membership, never stored"
    );
}

#[test]
fn a_not_started_non_member_has_no_outbound_state_label() {
    assert_eq!(
        outbound_state_label(&BoardState::NotStarted, false),
        None,
        "icebox is label-absence on the GitHub mirror — nothing to push, only stale labels to strip"
    );
}
