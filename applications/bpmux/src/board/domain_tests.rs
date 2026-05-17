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
