#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::evidence::*;
use crate::product_management::story::StoryId;
use chrono::Utc;
use tempfile::TempDir;

fn sample_verdict(story_id: u64, role: &str) -> Evidence {
    Evidence {
        story_id: StoryId::new(story_id),
        from_role: role.to_string(),
        verdict: "verified".to_string(),
        substrate_grounding: format!("grep-cited line at src/foo.rs:{story_id}"),
        ts: Utc::now(),
        grounded_head: None,
    }
}

#[test]
fn add_or_replace_verdict_inserts_when_role_absent_on_story() {
    let mut store = EvidenceStore::default();
    let outcome = add_or_replace_evidence(&mut store, sample_verdict(42, "designer"));
    assert_eq!(outcome, AddEvidenceOutcome::Inserted);
    assert_eq!(evidence_for_story(&store, StoryId::new(42)).len(), 1);
}

#[test]
fn add_or_replace_verdict_replaces_when_same_role_on_same_story() {
    let mut store = EvidenceStore::default();
    let first = sample_verdict(42, "designer");
    let mut second = sample_verdict(42, "designer");
    second.substrate_grounding = "updated cite".into();
    add_or_replace_evidence(&mut store, first);
    let outcome = add_or_replace_evidence(&mut store, second);
    assert_eq!(outcome, AddEvidenceOutcome::Replaced);
    let listed = evidence_for_story(&store, StoryId::new(42));
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].substrate_grounding, "updated cite");
}

#[test]
fn add_or_replace_verdict_keeps_distinct_roles_separate() {
    let mut store = EvidenceStore::default();
    add_or_replace_evidence(&mut store, sample_verdict(42, "designer"));
    add_or_replace_evidence(&mut store, sample_verdict(42, "dev"));
    let listed = evidence_for_story(&store, StoryId::new(42));
    assert_eq!(listed.len(), 2);
    let roles: Vec<&str> = listed.iter().map(|v| v.from_role.as_str()).collect();
    assert!(roles.contains(&"designer"));
    assert!(roles.contains(&"dev"));
}

#[test]
fn verdicts_for_unknown_story_returns_empty() {
    let store = EvidenceStore::default();
    assert!(evidence_for_story(&store, StoryId::new(999)).is_empty());
}

#[test]
fn save_load_round_trip_preserves_verdicts() {
    let dir = TempDir::new().unwrap();
    let mut store = EvidenceStore::default();
    add_or_replace_evidence(&mut store, sample_verdict(42, "designer"));
    add_or_replace_evidence(&mut store, sample_verdict(42, "dev"));
    add_or_replace_evidence(&mut store, sample_verdict(99, "designer"));
    save(dir.path(), &store).unwrap();
    let loaded = load(dir.path());
    assert_eq!(evidence_for_story(&loaded, StoryId::new(42)).len(), 2);
    assert_eq!(evidence_for_story(&loaded, StoryId::new(99)).len(), 1);
}

#[test]
fn load_returns_default_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let loaded = load(dir.path());
    assert!(loaded.stories.is_empty());
}
