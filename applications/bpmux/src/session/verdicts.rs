//! Per-story verdict storage — #1407 Slice 1.
//!
//! `submit_verdict` MCP-tool writes a `Verdict` to this canonical
//! substrate-data-store (sister to `pins` + `reactions` + `bookmarks`);
//! accept_story's evidence-check reads them out filtered by
//! story_number. Replaces the slice-3 permissive baseline
//! ("any recent message from role within window" — see acceptance.rs's
//! prior `recent_evidence_from_session`) with structured verdict-data
//! that enforces both SHAPE (this IS a verdict) and CONTENT
//! (`substrate_grounding` is a required field, not discipline-inside-
//! a-message-body).
//!
//! Persistence pattern mirrors `pins`: load from
//! `<session-dir>/verdicts.json` on session boot; rewrite on each
//! mutation. Sister-restart shape; no separate broadcast-axis (Slice 2
//! adds wake-tx broadcast if downstream renderer integration surfaces).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single verdict cast by a role on a story. The
/// `substrate_grounding` field carries the cite required for accept_
/// story's evidence-axis to clear — CONTENT enforced structurally
/// via the submit_verdict MCP-tool's required arg shape (Slice 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verdict {
    pub story_number: u64,
    pub from_role: String,
    pub verdict: String,
    pub substrate_grounding: String,
    pub ts: DateTime<Utc>,
}

/// All verdicts known to a session, keyed by story_number. Multiple
/// verdicts per story expected (one per evidence-axis role per fire).
/// Replaying the same (story, role) overwrites the prior entry —
/// retraction-shape is out-of-scope per #1407 body; last-write-wins
/// is the simplest substrate that satisfies the evidence-check
/// "any verdict from role within window" semantic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerdictsStore {
    pub stories: BTreeMap<u64, Vec<Verdict>>,
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

/// Record (or replay-over) a verdict for `(story_number, from_role)`.
/// Returns `true` if this was a new verdict for that (story, role)
/// pair; `false` if it replayed-over a prior verdict from the same
/// role on the same story (last-write-wins).
pub fn add_verdict(store: &mut VerdictsStore, verdict: Verdict) -> bool {
    let list = store.stories.entry(verdict.story_number).or_default();
    if let Some(existing) = list.iter_mut().find(|v| v.from_role == verdict.from_role) {
        *existing = verdict;
        false
    } else {
        list.push(verdict);
        true
    }
}

/// Return verdicts cast on `story_number`, in insertion order.
/// Used by the evidence-check caller to construct `EvidenceMessage`
/// entries for the workflow gate validator.
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
    fn add_verdict_inserts_new_role() {
        let mut store = VerdictsStore::default();
        let inserted = add_verdict(&mut store, sample_verdict(42, "designer"));
        assert!(inserted, "first verdict for a (story, role) is new");
        assert_eq!(verdicts_for_story(&store, 42).len(), 1);
    }

    #[test]
    fn add_verdict_replays_over_same_role_on_same_story() {
        let mut store = VerdictsStore::default();
        let first = sample_verdict(42, "designer");
        let mut second = sample_verdict(42, "designer");
        second.substrate_grounding = "updated cite".into();
        add_verdict(&mut store, first);
        let inserted = add_verdict(&mut store, second.clone());
        assert!(
            !inserted,
            "replay-over same (story, role) is not a new insertion"
        );
        let listed = verdicts_for_story(&store, 42);
        assert_eq!(listed.len(), 1, "replay-over keeps list length 1");
        assert_eq!(
            listed[0].substrate_grounding, "updated cite",
            "last-write-wins on (story, role) collision"
        );
    }

    #[test]
    fn add_verdict_keeps_distinct_roles_separate() {
        let mut store = VerdictsStore::default();
        add_verdict(&mut store, sample_verdict(42, "designer"));
        add_verdict(&mut store, sample_verdict(42, "dev"));
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
        add_verdict(&mut store, sample_verdict(42, "designer"));
        add_verdict(&mut store, sample_verdict(42, "dev"));
        add_verdict(&mut store, sample_verdict(99, "designer"));
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
