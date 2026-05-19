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

#[test]
fn story_state_github_label_canonical_form_per_variant() {
    assert_eq!(StoryState::Icebox.github_label(), "icebox");
    assert_eq!(StoryState::Ready.github_label(), "ready");
    assert_eq!(StoryState::InProgress.github_label(), "in progress");
    assert_eq!(StoryState::DeskCheck.github_label(), "desk-check");
    assert_eq!(StoryState::Delivered.github_label(), "delivered");
    assert_eq!(StoryState::Acceptance.github_label(), "acceptance");
    assert_eq!(StoryState::Accepted.github_label(), "accepted");
    assert_eq!(StoryState::Retired.github_label(), "retired");
}

#[test]
fn story_state_github_label_round_trips_via_state_from_labels() {
    for state in [
        StoryState::Ready,
        StoryState::InProgress,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Accepted,
        StoryState::Retired,
    ] {
        let label = state.github_label();
        let parsed = crate::board::adapters::github::state_from_labels(&[label.to_string()], false);
        assert_eq!(parsed, state, "state_from_labels round-trip for {state:?}");
    }
}

#[test]
fn story_state_github_label_differs_from_as_str_on_in_progress_and_matches_otherwise() {
    assert_ne!(
        StoryState::InProgress.github_label(),
        StoryState::InProgress.as_str(),
        "InProgress is the only variant where github_label (spaces) diverges from as_str (kebab)"
    );
    for state in [
        StoryState::Icebox,
        StoryState::Ready,
        StoryState::DeskCheck,
        StoryState::Delivered,
        StoryState::Acceptance,
        StoryState::Accepted,
        StoryState::Retired,
    ] {
        assert_eq!(
            state.github_label(),
            state.as_str(),
            "github_label and as_str agree for {state:?}"
        );
    }
}
