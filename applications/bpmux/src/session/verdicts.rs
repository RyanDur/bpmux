use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verdict {
    pub story_number: u64,
    pub from_role: String,
    pub verdict: String,
    pub substrate_grounding: String,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerdictsStore {
    pub stories: BTreeMap<u64, Vec<Verdict>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddVerdictOutcome {
    Inserted,
    Replaced,
}

pub fn verdicts_path(session_dir: &Path) -> PathBuf {
    session_dir.join("verdicts.json")
}

pub fn load(session_dir: &Path) -> VerdictsStore {
    let path = verdicts_path(session_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => VerdictsStore::default(),
    }
}

pub fn save(session_dir: &Path, store: &VerdictsStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = verdicts_path(session_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

pub fn add_or_replace_verdict(store: &mut VerdictsStore, verdict: Verdict) -> AddVerdictOutcome {
    let list = store.stories.entry(verdict.story_number).or_default();
    if let Some(existing) = list.iter_mut().find(|v| v.from_role == verdict.from_role) {
        *existing = verdict;
        AddVerdictOutcome::Replaced
    } else {
        list.push(verdict);
        AddVerdictOutcome::Inserted
    }
}

pub fn verdicts_for_story(store: &VerdictsStore, story_number: u64) -> Vec<Verdict> {
    store
        .stories
        .get(&story_number)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_verdict(story_number: u64, role: &str) -> Verdict {
        Verdict {
            story_number,
            from_role: role.to_string(),
            verdict: "verified".to_string(),
            substrate_grounding: format!("grep-cited line at src/foo.rs:{story_number}"),
            ts: Utc::now(),
        }
    }

    #[test]
    fn add_or_replace_verdict_inserts_when_role_absent_on_story() {
        let mut store = VerdictsStore::default();
        let outcome = add_or_replace_verdict(&mut store, sample_verdict(42, "designer"));
        assert_eq!(outcome, AddVerdictOutcome::Inserted);
        assert_eq!(verdicts_for_story(&store, 42).len(), 1);
    }

    #[test]
    fn add_or_replace_verdict_replaces_when_same_role_on_same_story() {
        let mut store = VerdictsStore::default();
        let first = sample_verdict(42, "designer");
        let mut second = sample_verdict(42, "designer");
        second.substrate_grounding = "updated cite".into();
        add_or_replace_verdict(&mut store, first);
        let outcome = add_or_replace_verdict(&mut store, second);
        assert_eq!(outcome, AddVerdictOutcome::Replaced);
        let listed = verdicts_for_story(&store, 42);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].substrate_grounding, "updated cite");
    }

    #[test]
    fn add_or_replace_verdict_keeps_distinct_roles_separate() {
        let mut store = VerdictsStore::default();
        add_or_replace_verdict(&mut store, sample_verdict(42, "designer"));
        add_or_replace_verdict(&mut store, sample_verdict(42, "dev"));
        let listed = verdicts_for_story(&store, 42);
        assert_eq!(listed.len(), 2);
        let roles: Vec<&str> = listed.iter().map(|v| v.from_role.as_str()).collect();
        assert!(roles.contains(&"designer"));
        assert!(roles.contains(&"dev"));
    }

    #[test]
    fn verdicts_for_unknown_story_returns_empty() {
        let store = VerdictsStore::default();
        assert!(verdicts_for_story(&store, 999).is_empty());
    }

    #[test]
    fn save_load_round_trip_preserves_verdicts() {
        let dir = TempDir::new().unwrap();
        let mut store = VerdictsStore::default();
        add_or_replace_verdict(&mut store, sample_verdict(42, "designer"));
        add_or_replace_verdict(&mut store, sample_verdict(42, "dev"));
        add_or_replace_verdict(&mut store, sample_verdict(99, "designer"));
        save(dir.path(), &store).unwrap();
        let loaded = load(dir.path());
        assert_eq!(verdicts_for_story(&loaded, 42).len(), 2);
        assert_eq!(verdicts_for_story(&loaded, 99).len(), 1);
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let loaded = load(dir.path());
        assert!(loaded.stories.is_empty());
    }
}
