use super::outbound_state_label;
use product_management::story::Work;

#[test]
fn the_outbound_label_is_the_coarse_position_never_a_fine_stage() {
    assert_eq!(
        outbound_state_label(&Work::InProgress, false),
        Some("in progress"),
        "de-leak: in-progress goes out coarse; the fine dev/desk-check stage stays local in placements.json, never on the label"
    );
    assert_eq!(
        outbound_state_label(&Work::Acceptance, false),
        Some("delivered")
    );
    assert_eq!(
        outbound_state_label(&Work::Accepted, false),
        Some("accepted")
    );
    assert_eq!(outbound_state_label(&Work::Retired, false), Some("retired"));
}

#[test]
fn a_ready_state_current_iteration_member_derives_the_ready_mirror_label() {
    assert_eq!(
        outbound_state_label(&Work::Ready, true),
        Some("ready"),
        "the human-readable GitHub mirror shows ready for a scheduled ready-state story — derived from membership, never stored"
    );
}

#[test]
fn a_ready_state_non_member_has_no_outbound_state_label() {
    assert_eq!(
        outbound_state_label(&Work::Ready, false),
        None,
        "icebox is label-absence on the GitHub mirror — nothing to push, only stale labels to strip"
    );
}
