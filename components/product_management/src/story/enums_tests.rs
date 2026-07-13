#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::enums::Work;
use super::stage::Stage;

#[test]
fn story_state_parse_round_trips_via_as_str() {
    for state in [
        Work::NotStarted,
        Work::InProgress,
        Work::Acceptance,
        Work::Accepted,
        Work::Retired,
    ] {
        let s = state.as_str();
        assert_eq!(Work::parse(s), Some(state), "parse({s:?}) round-trip");
    }
}

#[test]
fn story_state_parse_accepts_the_legacy_icebox_and_ready_strings_as_not_started() {
    assert_eq!(Work::parse("icebox"), Some(Work::NotStarted));
    assert_eq!(Work::parse("ready"), Some(Work::NotStarted));
    assert_eq!(Work::parse("not-started"), Some(Work::NotStarted));
}

#[test]
fn story_state_parse_accepts_underscore_variants() {
    assert_eq!(Work::parse("in_progress"), Some(Work::InProgress));
    assert_eq!(Work::parse("desk_check"), Some(Work::InProgress));
}

#[test]
fn story_state_parse_returns_none_for_unknown() {
    assert_eq!(Work::parse("not-a-state"), None);
    assert_eq!(Work::parse(""), None);
    assert_eq!(Work::parse("Ready"), None);
}

#[test]
fn story_state_serde_round_trips_via_canonical_string() {
    for state in [
        Work::NotStarted,
        Work::InProgress,
        Work::Acceptance,
        Work::Accepted,
        Work::Retired,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, format!("\"{}\"", state.as_str()));
        let back: Work = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state, "serde round-trip for {state:?}");
    }
}

#[test]
fn story_state_deserializes_persisted_legacy_icebox_and_ready_rows() {
    for legacy in ["\"icebox\"", "\"ready\""] {
        let back: Work = serde_json::from_str(legacy).unwrap();
        assert_eq!(
            back,
            Work::NotStarted,
            "persisted StoryCreated/StateTransitioned rows carrying {legacy} must keep deserializing after the collapse"
        );
    }
}

#[test]
fn story_state_serde_emits_canonical_kebab_not_variant_name() {
    assert_eq!(
        serde_json::to_string(&Work::InProgress).unwrap(),
        "\"in-progress\""
    );
    assert!(serde_json::from_str::<Work>("\"InProgress\"").is_err());
}

#[test]
fn story_state_acceptance_wire_vocab_parses_and_keeps_legacy_delivered_alias() {
    assert_eq!(Work::Acceptance.as_str(), "acceptance");
    assert_eq!(Work::parse("acceptance"), Some(Work::Acceptance));
    assert_eq!(Work::parse("delivered"), Some(Work::Acceptance));
}

#[test]
fn story_state_github_label_canonical_form_per_variant() {
    assert_eq!(
        Work::NotStarted.github_label(),
        None,
        "not-started has NO GitHub label at the type level — the outbound mirror derives ready-vs-absence from membership, and an empty-label push is unrepresentable"
    );
    assert_eq!(Work::InProgress.github_label(), Some("in progress"));
    assert_eq!(Work::Acceptance.github_label(), Some("delivered"));
    assert_eq!(Work::Accepted.github_label(), Some("accepted"));
    assert_eq!(Work::Retired.github_label(), Some("retired"));
}

#[test]
fn story_state_github_label_diverges_from_as_str_for_in_progress_and_acceptance() {
    assert_ne!(
        Work::InProgress.github_label().expect("labeled variant"),
        Work::InProgress.as_str(),
        "InProgress(Dev): github_label uses spaces, as_str uses kebab"
    );
    assert_ne!(
        Work::Acceptance.github_label().expect("labeled variant"),
        Work::Acceptance.as_str(),
        "Acceptance: the domain wire says `acceptance`, the GitHub edge keeps the historical `delivered` label"
    );
    for state in [Work::Accepted, Work::Retired] {
        assert_eq!(
            state.github_label().expect("labeled variant"),
            state.as_str(),
            "github_label and as_str agree for {state:?}"
        );
    }
}

#[test]
fn stage_id_bridges_the_typed_middle_stages_to_config_ids() {
    use workflow::StageId;
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
    let back: workflow::StageId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}
