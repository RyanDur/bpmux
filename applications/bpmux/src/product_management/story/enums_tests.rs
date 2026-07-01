#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::enums::BoardState;
use super::enums::Stage;

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev),
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Delivered,
        BoardState::Accepted,
    ] {
        let s = state.as_str();
        assert_eq!(BoardState::parse(s), Some(state), "parse({s:?}) round-trip");
    }
}

#[test]
fn story_state_parse_accepts_underscore_variants() {
    assert_eq!(
        BoardState::parse("in_progress"),
        Some(BoardState::InProgress(Stage::Dev))
    );
    assert_eq!(
        BoardState::parse("desk_check"),
        Some(BoardState::InProgress(Stage::DeskCheck))
    );
}

#[test]
fn story_state_parse_returns_none_for_unknown() {
    assert_eq!(BoardState::parse("not-a-state"), None);
    assert_eq!(BoardState::parse(""), None);
    assert_eq!(BoardState::parse("Ready"), None);
}

#[test]
fn story_state_serde_round_trips_via_canonical_string() {
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev),
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Delivered,
        BoardState::Accepted,
        BoardState::Retired,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, format!("\"{}\"", state.as_str()));
        let back: BoardState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state, "serde round-trip for {state:?}");
    }
}

#[test]
fn story_state_serde_emits_canonical_kebab_not_variant_name() {
    assert_eq!(
        serde_json::to_string(&BoardState::InProgress(Stage::Dev)).unwrap(),
        "\"in-progress\""
    );
    assert!(serde_json::from_str::<BoardState>("\"InProgress\"").is_err());
}

#[test]
fn story_state_delivered_legacy_label_preserved_for_backward_compat() {
    assert_eq!(BoardState::parse("delivered"), Some(BoardState::Delivered));
    assert_eq!(BoardState::Delivered.as_str(), "delivered");
}

#[test]
fn story_state_github_label_canonical_form_per_variant() {
    assert_eq!(BoardState::Icebox.github_label(), "icebox");
    assert_eq!(BoardState::Ready.github_label(), "ready");
    assert_eq!(
        BoardState::InProgress(Stage::Dev).github_label(),
        "in progress"
    );
    assert_eq!(
        BoardState::InProgress(Stage::DeskCheck).github_label(),
        "desk-check"
    );
    assert_eq!(BoardState::Delivered.github_label(), "delivered");
    assert_eq!(BoardState::Accepted.github_label(), "accepted");
    assert_eq!(BoardState::Retired.github_label(), "retired");
}

#[test]
fn story_state_github_label_round_trips_via_state_from_labels() {
    for state in [
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev),
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Delivered,
        BoardState::Accepted,
        BoardState::Retired,
    ] {
        let label = state.github_label();
        let parsed = crate::product_management::board::adapters::github::state_from_labels(
            &[label.to_string()],
            false,
        );
        assert_eq!(parsed, state, "state_from_labels round-trip for {state:?}");
    }
}

#[test]
fn story_state_github_label_differs_from_as_str_on_in_progress_and_matches_otherwise() {
    assert_ne!(
        BoardState::InProgress(Stage::Dev).github_label(),
        BoardState::InProgress(Stage::Dev).as_str(),
        "InProgress is the only variant where github_label (spaces) diverges from as_str (kebab)"
    );
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Delivered,
        BoardState::Accepted,
        BoardState::Retired,
    ] {
        assert_eq!(
            state.github_label(),
            state.as_str(),
            "github_label and as_str agree for {state:?}"
        );
    }
}
