use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::product_management::story::StoryId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub story_id: StoryId,
    pub from_role: String,
    pub verdict: String,
    pub substrate_grounding: String,
    pub ts: DateTime<Utc>,
    #[serde(default)]
    pub grounded_head: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceStore {
    pub stories: BTreeMap<StoryId, Vec<Evidence>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddEvidenceOutcome {
    Inserted,
    Replaced,
}

pub fn evidence_path(data_dir: &Path) -> PathBuf {
    data_dir.join("verdicts.json")
}

pub fn load(data_dir: &Path) -> EvidenceStore {
    let path = evidence_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => EvidenceStore::default(),
    }
}

pub fn save(data_dir: &Path, store: &EvidenceStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(data_dir)?;
    let path = evidence_path(data_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

pub fn add_or_replace_evidence(
    store: &mut EvidenceStore,
    evidence: Evidence,
) -> AddEvidenceOutcome {
    let list = store.stories.entry(evidence.story_id).or_default();
    if let Some(existing) = list.iter_mut().find(|v| v.from_role == evidence.from_role) {
        *existing = evidence;
        AddEvidenceOutcome::Replaced
    } else {
        list.push(evidence);
        AddEvidenceOutcome::Inserted
    }
}

pub fn evidence_for_story(store: &EvidenceStore, story_id: StoryId) -> Vec<Evidence> {
    store.stories.get(&story_id).cloned().unwrap_or_default()
}
