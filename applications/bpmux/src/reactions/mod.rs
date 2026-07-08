pub mod slice;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reaction {
    pub message_key: String,
    pub emoji: String,
    pub user: String,
    pub reacted_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReactionsStore {
    pub channels: BTreeMap<String, Vec<Reaction>>,
}

pub fn reactions_path(data_dir: &Path) -> PathBuf {
    data_dir.join("reactions.json")
}

pub fn load(data_dir: &Path) -> ReactionsStore {
    let path = reactions_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ReactionsStore::default(),
    }
}

pub fn save(data_dir: &Path, store: &ReactionsStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(data_dir)?;
    let path = reactions_path(data_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}
