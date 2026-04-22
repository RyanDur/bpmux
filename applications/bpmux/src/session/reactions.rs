//! Per-message emoji reactions — #340.
//!
//! Any viewer can add/remove reactions on any message. Reactions
//! survive restart via `<session-dir>/reactions.json`, broadcast
//! via REACTIONS_UPDATE_MARKER so peer renderers see pill updates
//! within WS latency.
//!
//! `channel` follows the same canonical-key derivation as pins —
//! see session::pins::channel_key. `message_key` = `{ts}|{from}`.
//!
//! Note on AC6 ("persist in the JSONL log"): this impl stores
//! reactions in a separate json file rather than inline in the
//! append-only messages-current.jsonl. The message log is append-
//! only; reactions are mutable per-message state. Separate store
//! keeps the log pristine while satisfying persist-across-restart.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single reaction instance. Multiple Reactions with the same
/// (message_key, emoji) across different users aggregate to a
/// single pill with the per-user list for the hover tooltip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reaction {
    pub message_key: String,
    pub emoji: String,
    pub user: String,
    pub reacted_at: String,
}

/// Aggregated reaction shape the renderer consumes: one entry per
/// (message_key, emoji) with a list of reactors + count. Frontends
/// can render without recomputing the aggregation.
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedReaction {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

/// Full reactions store: channel_key → flat list of Reactions.
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

/// Toggle a reaction: adds if the (user, emoji) combo doesn't exist
/// on this message; removes if it does. Returns the new state:
/// true = reaction present after toggle, false = removed.
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

/// Aggregate by (message_key, emoji) for a channel. Renderer
/// consumes this shape directly.
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn toggle_adds_then_removes_for_same_user_emoji() {
        let mut store = ReactionsStore::default();
        assert!(toggle_reaction(
            &mut store,
            "broadcast",
            "k1",
            "👍",
            "ryan",
            "t0"
        ));
        assert!(!toggle_reaction(
            &mut store,
            "broadcast",
            "k1",
            "👍",
            "ryan",
            "t1"
        ));
        assert_eq!(store.channels.get("broadcast"), None);
    }

    #[test]
    fn different_users_aggregate_under_same_emoji() {
        let mut store = ReactionsStore::default();
        toggle_reaction(&mut store, "broadcast", "k1", "👍", "ryan", "t0");
        toggle_reaction(&mut store, "broadcast", "k1", "👍", "dev", "t1");
        let agg = aggregate_for_channel(&store, "broadcast");
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].count, 2);
        assert!(agg[0].users.contains(&"ryan".to_string()));
        assert!(agg[0].users.contains(&"dev".to_string()));
    }

    #[test]
    fn different_emojis_aggregate_separately() {
        let mut store = ReactionsStore::default();
        toggle_reaction(&mut store, "broadcast", "k1", "👍", "ryan", "t0");
        toggle_reaction(&mut store, "broadcast", "k1", "🎉", "ryan", "t1");
        let agg = aggregate_for_channel(&store, "broadcast");
        assert_eq!(agg.len(), 2);
    }

    #[test]
    fn save_and_load_roundtrips() {
        let dir = tempdir().unwrap();
        let mut store = ReactionsStore::default();
        toggle_reaction(&mut store, "broadcast", "k1", "👍", "ryan", "t0");
        save(dir.path(), &store).unwrap();
        let loaded = load(dir.path());
        let agg = aggregate_for_channel(&loaded, "broadcast");
        assert_eq!(agg[0].users, vec!["ryan".to_string()]);
    }
}
