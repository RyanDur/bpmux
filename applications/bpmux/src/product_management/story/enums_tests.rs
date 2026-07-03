#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::enums::BoardState;
use super::stage::Stage;

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev),
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Acceptance,
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
        BoardState::Acceptance,
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
fn story_state_acceptance_wire_vocab_parses_and_keeps_legacy_delivered_alias() {
    assert_eq!(BoardState::Acceptance.as_str(), "acceptance");
    assert_eq!(
        BoardState::parse("acceptance"),
        Some(BoardState::Acceptance)
    );
    assert_eq!(BoardState::parse("delivered"), Some(BoardState::Acceptance));
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
    assert_eq!(BoardState::Acceptance.github_label(), "delivered");
    assert_eq!(BoardState::Accepted.github_label(), "accepted");
    assert_eq!(BoardState::Retired.github_label(), "retired");
}

#[test]
fn story_state_github_label_round_trips_via_state_from_labels() {
    for state in [
        BoardState::Ready,
        BoardState::InProgress(Stage::Dev),
        BoardState::InProgress(Stage::DeskCheck),
        BoardState::Acceptance,
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
fn story_state_github_label_diverges_from_as_str_for_in_progress_and_acceptance() {
    assert_ne!(
        BoardState::InProgress(Stage::Dev).github_label(),
        BoardState::InProgress(Stage::Dev).as_str(),
        "InProgress(Dev): github_label uses spaces, as_str uses kebab"
    );
    assert_ne!(
        BoardState::Acceptance.github_label(),
        BoardState::Acceptance.as_str(),
        "Acceptance: the domain wire says `acceptance`, the GitHub edge keeps the historical `delivered` label"
    );
    for state in [
        BoardState::Icebox,
        BoardState::Ready,
        BoardState::InProgress(Stage::DeskCheck),
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

#[test]
fn stage_id_bridges_the_typed_middle_stages_to_config_ids() {
    use super::stage::StageId;
    assert_eq!(Stage::Dev.id().as_str(), "in-progress");
    assert_eq!(Stage::DeskCheck.id().as_str(), "desk-check");
    assert_eq!(StageId::from("in-progress").as_stage(), Some(Stage::Dev));
    assert_eq!(
        StageId::from("desk-check").as_stage(),
        Some(Stage::DeskCheck)
    );
    assert_eq!(StageId::from("qa").as_stage(), None);
}

#[test]
fn stage_id_round_trips_through_its_string_id() {
    let id = Stage::DeskCheck.id();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"desk-check\"");
    let back: super::stage::StageId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}
