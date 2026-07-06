use super::outbound_state_label;
use crate::product_management::story::BoardState;

#[test]
fn the_outbound_label_is_the_coarse_position_never_a_fine_stage() {
    assert_eq!(
        outbound_state_label(&BoardState::InProgress),
        "in progress",
        "de-leak: in-progress goes out coarse; the fine dev/desk-check stage stays local in placements.json, never on the label"
    );
    assert_eq!(outbound_state_label(&BoardState::Ready), "ready");
    assert_eq!(outbound_state_label(&BoardState::Acceptance), "delivered");
    assert_eq!(outbound_state_label(&BoardState::Accepted), "accepted");
    assert_eq!(outbound_state_label(&BoardState::Retired), "retired");
}
