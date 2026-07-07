#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::enums::BoardState;
use super::stage::Stage;

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        BoardState::NotStarted,
        BoardState::InProgress,
        BoardState::Acceptance,
        BoardState::Accepted,
        BoardState::Retired,
    ] {
        let s = state.as_str();
        assert_eq!(BoardState::parse(s), Some(state), "parse({s:?}) round-trip");
    }
}

#[test]
fn story_state_parse_accepts_the_legacy_icebox_and_ready_strings_as_not_started() {
    assert_eq!(BoardState::parse("icebox"), Some(BoardState::NotStarted));
    assert_eq!(BoardState::parse("ready"), Some(BoardState::NotStarted));
    assert_eq!(
        BoardState::parse("not-started"),
        Some(BoardState::NotStarted)
    );
}

#[test]
fn story_state_parse_accepts_underscore_variants() {
    assert_eq!(
        BoardState::parse("in_progress"),
        Some(BoardState::InProgress)
    );
    assert_eq!(
        BoardState::parse("desk_check"),
        Some(BoardState::InProgress)
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
        BoardState::NotStarted,
        BoardState::InProgress,
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
fn story_state_deserializes_persisted_legacy_icebox_and_ready_rows() {
    for legacy in ["\"icebox\"", "\"ready\""] {
        let back: BoardState = serde_json::from_str(legacy).unwrap();
        assert_eq!(
            back,
            BoardState::NotStarted,
            "persisted StoryCreated/StateTransitioned rows carrying {legacy} must keep deserializing after the collapse"
        );
    }
}

#[test]
fn story_state_serde_emits_canonical_kebab_not_variant_name() {
    assert_eq!(
        serde_json::to_string(&BoardState::InProgress).unwrap(),
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
    assert_eq!(
        BoardState::NotStarted.github_label(),
        "",
        "not-started has NO static GitHub label — the outbound mirror derives ready-vs-absence from membership"
    );
    assert_eq!(BoardState::InProgress.github_label(), "in progress");
    assert_eq!(BoardState::Acceptance.github_label(), "delivered");
    assert_eq!(BoardState::Accepted.github_label(), "accepted");
    assert_eq!(BoardState::Retired.github_label(), "retired");
}

#[test]
fn story_state_github_labels_round_trip_except_not_started_which_is_membership_derived() {
    for state in [
        BoardState::InProgress,
        BoardState::Acceptance,
        BoardState::Accepted,
        BoardState::Retired,
    ] {
        let label = state.github_label();
        let parsed =
            crate::integrations::github::mapper::state_from_labels(&[label.to_string()], false);
        assert_eq!(parsed, state, "state_from_labels round-trip for {state:?}");
    }
    assert_eq!(
        crate::integrations::github::mapper::state_from_labels(&["ready".to_string()], false),
        BoardState::NotStarted,
        "the ready label is outbound-only — scheduling decodes from iteration membership, so the label alone falls through to the not-started fallback"
    );
}

#[test]
fn story_state_github_label_diverges_from_as_str_for_in_progress_and_acceptance() {
    assert_ne!(
        BoardState::InProgress.github_label(),
        BoardState::InProgress.as_str(),
        "InProgress(Dev): github_label uses spaces, as_str uses kebab"
    );
    assert_ne!(
        BoardState::Acceptance.github_label(),
        BoardState::Acceptance.as_str(),
        "Acceptance: the domain wire says `acceptance`, the GitHub edge keeps the historical `delivered` label"
    );
    for state in [BoardState::Accepted, BoardState::Retired] {
        assert_eq!(
            state.github_label(),
            state.as_str(),
            "github_label and as_str agree for {state:?}"
        );
    }
}

#[test]
fn stage_id_bridges_the_typed_middle_stages_to_config_ids() {
    use crate::workflow::StageId;
    assert_eq!(Stage::Dev.id().as_str(), "in-progress");
    assert_eq!(Stage::DeskCheck.id().as_str(), "desk-check");
    assert_eq!(
        Stage::from_id(&StageId::from("in-progress")),
        Some(Stage::Dev)
    );
    assert_eq!(
        Stage::from_id(&StageId::from("desk-check")),
        Some(Stage::DeskCheck)
    );
    assert_eq!(Stage::from_id(&StageId::from("qa")), None);
}

#[test]
fn stage_id_round_trips_through_its_string_id() {
    let id = Stage::DeskCheck.id();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"desk-check\"");
    let back: crate::workflow::StageId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}
