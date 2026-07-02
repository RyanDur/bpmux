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

#[derive(Debug, Clone, Serialize)]
pub struct AggregatedReaction {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReactionsStore {
    pub channels: BTreeMap<String, Vec<Reaction>>,
}

pub fn reactions_path(session_dir: &Path) -> PathBuf {
    session_dir.join("reactions.json")
}

pub fn load(session_dir: &Path) -> ReactionsStore {
    let path = reactions_path(session_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ReactionsStore::default(),
    }
}

pub fn save(session_dir: &Path, store: &ReactionsStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = reactions_path(session_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

pub fn toggle_reaction(
    store: &mut ReactionsStore,
    channel: &str,
    message_key: &str,
    emoji: &str,
    user: &str,
    now: &str,
) -> bool {
    let list = store.channels.entry(channel.to_string()).or_default();
    let existing_idx = list
        .iter()
        .position(|r| r.message_key == message_key && r.emoji == emoji && r.user == user);
    if let Some(idx) = existing_idx {
        list.remove(idx);
        if list.is_empty() {
            store.channels.remove(channel);
        }
        false
    } else {
        list.push(Reaction {
            message_key: message_key.to_string(),
            emoji: emoji.to_string(),
            user: user.to_string(),
            reacted_at: now.to_string(),
        });
        true
    }
}

pub fn aggregate_for_channel(store: &ReactionsStore, channel: &str) -> Vec<AggregatedReaction> {
    let Some(list) = store.channels.get(channel) else {
        return Vec::new();
    };
    let mut buckets: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for r in list {
        buckets
            .entry((r.message_key.clone(), r.emoji.clone()))
            .or_default()
            .push(r.user.clone());
    }
    buckets
        .into_iter()
        .map(|((message_key, emoji), users)| AggregatedReaction {
            count: users.len(),
            message_key,
            emoji,
            users,
        })
        .collect()
}
