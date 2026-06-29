use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::product_management::story::StoryId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verdict {
    pub story_id: StoryId,
    pub from_role: String,
    pub verdict: String,
    pub substrate_grounding: String,
    pub ts: DateTime<Utc>,
    #[serde(default)]
    pub grounded_head: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerdictsStore {
    pub stories: BTreeMap<StoryId, Vec<Verdict>>,
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
    let list = store.stories.entry(verdict.story_id).or_default();
    if let Some(existing) = list.iter_mut().find(|v| v.from_role == verdict.from_role) {
        *existing = verdict;
        AddVerdictOutcome::Replaced
    } else {
        list.push(verdict);
        AddVerdictOutcome::Inserted
    }
}

pub fn verdicts_for_story(store: &VerdictsStore, story_id: StoryId) -> Vec<Verdict> {
    store.stories.get(&story_id).cloned().unwrap_or_default()
}
