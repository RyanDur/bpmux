#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use product_management::story::Work;

#[test]
fn story_state_github_labels_round_trip_except_ready_which_is_membership_derived() {
    for state in [
        Work::InProgress,
        Work::Acceptance,
        Work::Accepted,
        Work::Retired,
    ] {
        let label = state.github_label().expect("labeled variant");
        let parsed = source_codec::mapper::state_from_labels(&[label.to_string()], false);
        assert_eq!(parsed, state, "state_from_labels round-trip for {state:?}");
    }
    assert_eq!(
        source_codec::mapper::state_from_labels(&["ready".to_string()], false),
        Work::Ready,
        "the ready label is outbound-only — scheduling decodes from iteration membership, so the label alone falls through to the ready-state fallback"
    );
}
