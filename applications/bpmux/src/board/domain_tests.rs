#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::domain::StoryState;

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Acceptance,
        StoryState::Accepted,
    ] {
        let s = state.as_str();
        assert_eq!(StoryState::parse(s), Some(state), "parse({s:?}) round-trip");
    }
}

#[test]
fn story_state_parse_accepts_underscore_variants() {
    assert_eq!(
        StoryState::parse("in_progress"),
        Some(StoryState::InProgress)
    );
    assert_eq!(StoryState::parse("desk_check"), Some(StoryState::DeskCheck));
}

#[test]
fn story_state_parse_returns_none_for_unknown() {
    assert_eq!(StoryState::parse("not-a-state"), None);
    assert_eq!(StoryState::parse(""), None);
    assert_eq!(StoryState::parse("Ready"), None);
}

#[test]
fn story_state_acceptance_variant_parses_canonically() {
    assert_eq!(
        StoryState::parse("acceptance"),
        Some(StoryState::Acceptance)
    );
    assert_eq!(StoryState::Acceptance.as_str(), "acceptance");
}

#[test]
fn story_state_delivered_legacy_label_preserved_for_backward_compat() {
    assert_eq!(StoryState::parse("delivered"), Some(StoryState::Delivered));
    assert_eq!(StoryState::Delivered.as_str(), "delivered");
}
