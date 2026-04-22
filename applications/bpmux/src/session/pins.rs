//! Per-channel pinned-message storage — #342.
//!
//! Any viewer can pin a message to its channel (broadcast / DM /
//! group). Pins survive restart via `<session-dir>/pins.json` and
//! broadcast via BOARD_INVALIDATE_MARKER-style marker so peers see
//! each other's pin state live.
//!
//! `channel` is a stable string key — "broadcast" for to=null/all,
//! a single recipient name for DMs, or a sorted-joined list for
//! groups (e.g., "designer+dev+pm"). Renderer and backend MUST
//! derive the same key; `channel_key` is the canonical helper.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single pinned message. Denormalizes the message fields so the
/// pinned-list overlay can render without re-fetching the original
/// message from the log (which may have been compacted/rotated).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pin {
    /// Stable per-message key: `"{ts}|{from}"`. Scoped per-channel so
    /// the same message in broadcast + a DM (rare, but possible with
    /// reposts) can be pinned independently.
    pub message_key: String,
    /// Author of the pinned message (full identity e.g. "dev:claude-code").
    pub from: String,
    /// Timestamp of the pinned message.
    pub ts: String,
    /// Body of the pinned message.
    pub text: String,
    /// Role (short name) that pinned it.
    pub pinned_by: String,
    /// When the pin was created.
    pub pinned_at: String,
}

/// Full pins map for a session: channel_key → ordered list of pins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PinsStore {
    pub channels: BTreeMap<String, Vec<Pin>>,
}

/// Canonical channel-key derivation. Keep in sync with the renderer's
/// `channelKey()` in hooks/usePins.ts.
pub fn channel_key(to: Option<&[String]>) -> String {
    let Some(list) = to else {
        return "broadcast".to_string();
    };
    if list.is_empty() || list.iter().any(|n| n == "all") {
        return "broadcast".to_string();
    }
    let mut sorted: Vec<String> = list.to_vec();
    sorted.sort();
    sorted.join("+")
}

pub fn pins_path(session_dir: &Path) -> PathBuf {
    session_dir.join("pins.json")
}

pub fn load(session_dir: &Path) -> PinsStore {
    let path = pins_path(session_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => PinsStore::default(),
    }
}

pub fn save(session_dir: &Path, store: &PinsStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = pins_path(session_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

/// Add a pin to a channel. Idempotent — pinning an already-pinned
/// message is a no-op, not an error. Returns true if newly added.
pub fn add_pin(store: &mut PinsStore, channel: &str, pin: Pin) -> bool {
    let list = store.channels.entry(channel.to_string()).or_default();
    if list.iter().any(|p| p.message_key == pin.message_key) {
        return false;
    }
    list.push(pin);
    true
}

/// Remove a pin. Returns true if found + removed.
pub fn remove_pin(store: &mut PinsStore, channel: &str, message_key: &str) -> bool {
    let Some(list) = store.channels.get_mut(channel) else {
        return false;
    };
    let before = list.len();
    list.retain(|p| p.message_key != message_key);
    let removed = list.len() != before;
    let now_empty = list.is_empty();
    if now_empty {
        store.channels.remove(channel);
    }
    removed
}

/// List pins for a channel (empty if none).
pub fn list_pins(store: &PinsStore, channel: &str) -> Vec<Pin> {
    store.channels.get(channel).cloned().unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn pin(key: &str, from: &str) -> Pin {
        Pin {
            message_key: key.to_string(),
            from: from.to_string(),
            ts: "2026-04-22T00:00:00Z".to_string(),
            text: "test".to_string(),
            pinned_by: "dev".to_string(),
            pinned_at: "2026-04-22T01:00:00Z".to_string(),
        }
    }

    #[test]
    fn channel_key_for_broadcast_variants() {
        assert_eq!(channel_key(None), "broadcast");
        assert_eq!(channel_key(Some(&[])), "broadcast");
        assert_eq!(channel_key(Some(&["all".to_string()])), "broadcast");
    }

    #[test]
    fn channel_key_sorts_group_names_for_stability() {
        let a = channel_key(Some(&["dev".to_string(), "designer".to_string()]));
        let b = channel_key(Some(&["designer".to_string(), "dev".to_string()]));
        assert_eq!(a, b);
        assert_eq!(a, "designer+dev");
    }

    #[test]
    fn add_pin_is_idempotent_by_message_key() {
        let mut store = PinsStore::default();
        assert!(add_pin(&mut store, "broadcast", pin("k1", "ryan")));
        assert!(!add_pin(&mut store, "broadcast", pin("k1", "ryan")));
        assert_eq!(list_pins(&store, "broadcast").len(), 1);
    }

    #[test]
    fn remove_pin_clears_empty_channel_entry() {
        let mut store = PinsStore::default();
        add_pin(&mut store, "broadcast", pin("k1", "ryan"));
        assert!(remove_pin(&mut store, "broadcast", "k1"));
        assert!(!store.channels.contains_key("broadcast"));
    }

    #[test]
    fn save_and_load_roundtrips() {
        let dir = tempdir().unwrap();
        let mut store = PinsStore::default();
        add_pin(&mut store, "broadcast", pin("k1", "ryan"));
        add_pin(&mut store, "designer+dev", pin("k2", "designer"));
        save(dir.path(), &store).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.channels.len(), 2);
        assert_eq!(list_pins(&loaded, "broadcast")[0].message_key, "k1");
    }
}
