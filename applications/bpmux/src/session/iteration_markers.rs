use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationMarker {
    pub id: u64,
    pub theme_name: String,
    pub position: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IterationMarkersStore {
    pub markers: Vec<IterationMarker>,
    pub next_id: u64,
}

pub fn iteration_markers_path(session_dir: &Path) -> PathBuf {
    session_dir.join("iteration_markers.json")
}

pub fn load(session_dir: &Path) -> IterationMarkersStore {
    let path = iteration_markers_path(session_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => IterationMarkersStore::default(),
    }
}

pub fn save(session_dir: &Path, store: &IterationMarkersStore) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = iteration_markers_path(session_dir);
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

pub fn list(store: &IterationMarkersStore) -> Vec<IterationMarker> {
    let mut out = store.markers.clone();
    out.sort_by_key(|m| m.position);
    out
}

pub fn add(
    store: &mut IterationMarkersStore,
    theme_name: String,
    created_at: String,
) -> IterationMarker {
    let id = store.next_id;
    store.next_id += 1;
    let position = store
        .markers
        .iter()
        .map(|m| m.position)
        .max()
        .map(|p| p + 1)
        .unwrap_or(0);
    let marker = IterationMarker {
        id,
        theme_name,
        position,
        created_at,
    };
    store.markers.push(marker.clone());
    marker
}

pub fn remove(store: &mut IterationMarkersStore, id: u64) -> bool {
    let before = store.markers.len();
    store.markers.retain(|m| m.id != id);
    store.markers.len() != before
}

/// #1271: evict markers from the store whose id is NOT in the
/// canonical icebox_unified. Returns the number of orphans evicted.
/// Sister of `priority::auto_heal_icebox_unified_via_workflow_state`
/// at markers-store-axis — the canonical positional substrate is
/// `icebox_unified`, and the markers-store is a downstream mirror
/// of marker metadata. When close_iteration / delete_iteration_marker
/// fires the positional removal but the store-side cleanup is lost
/// (boot-restart, state-transition gap, fixture-setup), this heal
/// reconciles the divergence.
///
/// Markers IN icebox_unified are preserved unconditionally. Idempotent.
pub fn auto_heal_orphans(
    store: &mut IterationMarkersStore,
    canonical_ids: &std::collections::HashSet<u64>,
) -> usize {
    let before = store.markers.len();
    store.markers.retain(|m| canonical_ids.contains(&m.id));
    before - store.markers.len()
}

pub fn next_unused_theme(store: &IterationMarkersStore) -> String {
    let used: std::collections::HashSet<&String> =
        store.markers.iter().map(|m| &m.theme_name).collect();
    for theme in crate::orchestrator::iteration_markers::CONSTELLATION_THEME_POOL {
        if !used.contains(&theme.to_string()) {
            return theme.to_string();
        }
    }
    format!("Marker {}", store.markers.len() + 1)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_roundtrips() {
        let dir = tempdir().unwrap();
        let mut store = IterationMarkersStore::default();
        add(
            &mut store,
            "Orion".to_string(),
            "2026-05-09T19:00:00Z".to_string(),
        );
        add(
            &mut store,
            "Lyra".to_string(),
            "2026-05-09T19:01:00Z".to_string(),
        );
        save(dir.path(), &store).unwrap();
        let loaded = load(dir.path());
        assert_eq!(loaded.markers.len(), 2);
        assert_eq!(loaded.next_id, 2);
        assert_eq!(loaded.markers[0].theme_name, "Orion");
    }

    #[test]
    fn add_assigns_monotonic_ids_and_positions() {
        let mut store = IterationMarkersStore::default();
        let a = add(&mut store, "Orion".to_string(), "ts".to_string());
        let b = add(&mut store, "Lyra".to_string(), "ts".to_string());
        assert_eq!(a.id, 0);
        assert_eq!(a.position, 0);
        assert_eq!(b.id, 1);
        assert_eq!(b.position, 1);
    }

    #[test]
    fn remove_drops_marker_returns_true_when_found() {
        let mut store = IterationMarkersStore::default();
        let m = add(&mut store, "Orion".to_string(), "ts".to_string());
        assert!(remove(&mut store, m.id));
        assert!(store.markers.is_empty());
    }

    #[test]
    fn remove_unknown_id_returns_false() {
        let mut store = IterationMarkersStore::default();
        assert!(!remove(&mut store, 99));
    }

    #[test]
    fn list_returns_markers_sorted_by_position() {
        let mut store = IterationMarkersStore::default();
        add(&mut store, "Orion".to_string(), "ts".to_string());
        add(&mut store, "Lyra".to_string(), "ts".to_string());
        let listed = list(&store);
        assert_eq!(listed.len(), 2);
        assert!(listed[0].position < listed[1].position);
    }

    #[test]
    fn next_unused_theme_picks_first_unused_constellation() {
        let mut store = IterationMarkersStore::default();
        let first = next_unused_theme(&store);
        add(&mut store, first.clone(), "ts".to_string());
        let second = next_unused_theme(&store);
        assert_ne!(first, second);
    }
}
