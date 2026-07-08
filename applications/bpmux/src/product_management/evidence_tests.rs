#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::evidence::{
    EvidenceAction, EvidenceState, evidence_for_story, from_store, load, reduce, save, to_store,
};
use crate::product_management::story::StoryId;
use chrono::Utc;
use tempfile::TempDir;

fn submit(story_id: u64, role: &str) -> EvidenceAction {
    EvidenceAction::VerdictSubmitted {
        story_id: StoryId::new(story_id),
        from_role: role.to_string(),
        verdict: "verified".to_string(),
        substrate_grounding: format!("grep-cited line at src/foo.rs:{story_id}"),
        ts: Utc::now(),
        grounded_head: None,
    }
}

fn apply(state: EvidenceState, action: &EvidenceAction) -> EvidenceState {
    reduce(&state, action).expect("VerdictSubmitted yields a new state")
}

#[test]
fn verdict_submitted_inserts_when_role_absent_on_story() {
    let state = apply(EvidenceState::default(), &submit(42, "designer"));
    assert_eq!(evidence_for_story(&state, StoryId::new(42)).len(), 1);
}

#[test]
fn verdict_submitted_replaces_when_same_role_on_same_story() {
    let mut state = apply(EvidenceState::default(), &submit(42, "designer"));
    let second = EvidenceAction::VerdictSubmitted {
        story_id: StoryId::new(42),
        from_role: "designer".to_string(),
        verdict: "verified".to_string(),
        substrate_grounding: "updated cite".to_string(),
        ts: Utc::now(),
        grounded_head: None,
    };
    state = apply(state, &second);
    let listed = evidence_for_story(&state, StoryId::new(42));
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].substrate_grounding, "updated cite");
}

#[test]
fn verdict_submitted_keeps_distinct_roles_separate() {
    let mut state = apply(EvidenceState::default(), &submit(42, "designer"));
    state = apply(state, &submit(42, "dev"));
    let listed = evidence_for_story(&state, StoryId::new(42));
    assert_eq!(listed.len(), 2);
    let roles: Vec<&str> = listed.iter().map(|v| v.from_role.as_str()).collect();
    assert!(roles.contains(&"designer"));
    assert!(roles.contains(&"dev"));
}

#[test]
fn evidence_for_unknown_story_returns_empty() {
    let state = EvidenceState::default();
    assert!(evidence_for_story(&state, StoryId::new(999)).is_empty());
}

#[test]
fn save_then_load_is_lossless_including_grounded_head_and_ts() {
    let dir = TempDir::new().unwrap();
    let ts = Utc::now();
    let head = "a-frozen-sha".to_string();
    let action = EvidenceAction::VerdictSubmitted {
        story_id: StoryId::new(42),
        from_role: "designer".to_string(),
        verdict: "verified".to_string(),
        substrate_grounding: "src/foo.rs cite".to_string(),
        ts,
        grounded_head: Some(head.clone()),
    };
    let state = apply(EvidenceState::default(), &action);

    save(dir.path(), &to_store(&state)).unwrap();
    let rehydrated = from_store(&load(dir.path()));

    let listed = evidence_for_story(&rehydrated, StoryId::new(42));
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].ts, ts);
    assert_eq!(listed[0].grounded_head.as_deref(), Some("a-frozen-sha"));
    assert_eq!(
        listed,
        evidence_for_story(&state, StoryId::new(42)),
        "round-trip through the on-disk shape preserves the whole evidence record"
    );
}

#[test]
fn load_returns_default_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let loaded = from_store(&load(dir.path()));
    assert!(loaded.stories.is_empty());
}
