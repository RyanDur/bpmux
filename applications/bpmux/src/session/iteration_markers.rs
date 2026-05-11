use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationMarker {
    pub id: u64,
    pub theme_name: String,
    pub position: u32,
    pub created_at: String,
    /// #1288: snapshot of story-ids folded above this marker in
    /// icebox_unified, captured at the most-recent
    /// `add_stories_to_iteration` / `remove_stories_from_iteration`
    /// call. Used by the boot-time `restore_story_fold_positions_to_
    /// icebox_unified` heal to repair marker→stories adjacency post-
    /// restart. `#[serde(default)]` for backward-compat: legacy
    /// iteration_markers.json files with no `stories` field load as
    /// an empty Vec (no fold-snapshot ⇒ boot-heal no-op for that
    /// marker).
    ///
    /// Order: top-of-fold first, just-above-marker last. So fold
    /// `[100, 101, 102]` means icebox_unified contains story 100,
    /// then story 101, then story 102, then this marker.
    #[serde(default)]
    pub stories: Vec<u64>,
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
        stories: Vec::new(),
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

/// #1288: refresh the `stories` snapshot on every marker in `store` to
/// match the current positional-adjacency in `icebox_unified`. For
/// each marker M present in icebox_unified, M.stories becomes the list
/// of story-ids that appear contiguously above M (between M and the
/// previous marker — or the start of icebox_unified if M is the first
/// marker). Markers absent from icebox_unified get their fold cleared
/// (consistent with the rule that fold-snapshot reflects current
/// adjacency, not historical).
///
/// Call this AFTER every code path that mutates marker→story
/// adjacency in icebox_unified (`add_stories_to_iteration`,
/// `remove_stories_from_iteration`, `reorder_iteration_marker`,
/// `auto_heal_icebox`). The fold-snapshot is the truth-source that
/// `restore_story_fold_positions_to_icebox_unified` reads at boot
/// to repair adjacency post-restart.
pub fn sync_marker_fold_snapshots(
    store: &mut IterationMarkersStore,
    icebox_unified: &[super::priority::IceboxItem],
) {
    for marker in &mut store.markers {
        marker.stories = compute_fold_above_marker(icebox_unified, marker.id);
    }
}

/// #1294: backfill `stories` snapshot for legacy markers whose
/// `stories[]` is empty by deriving from icebox_unified positional-
/// adjacency. Idempotent: markers with non-empty stories[] are NOT
/// overwritten (preserves forward-going #1288 sync output).
///
/// Distinct from `sync_marker_fold_snapshots` which OVERWRITES every
/// marker's stories[] (including clearing on absent-from-icebox).
/// Backfill is one-shot migration-axis: legacy markers created before
/// #1288's schema-add have empty stories[]; this populates them from
/// the canonical icebox_unified positional substrate at boot, so
/// pre-#1288 iteration-plan substrate becomes visible after restart.
///
/// Returns count of markers backfilled.
pub fn backfill_legacy_marker_stories(
    store: &mut IterationMarkersStore,
    icebox_unified: &[super::priority::IceboxItem],
) -> usize {
    let mut backfilled = 0;
    for marker in &mut store.markers {
        if !marker.stories.is_empty() {
            continue;
        }
        let derived = compute_fold_above_marker(icebox_unified, marker.id);
        if !derived.is_empty() {
            marker.stories = derived;
            backfilled += 1;
        }
    }
    backfilled
}

/// Pure: extract the list of story-ids contiguously preceding the
/// given marker in icebox_unified. Returns empty Vec if marker is
/// absent. Order: top-of-fold first, just-above-marker last.
pub fn compute_fold_above_marker(
    icebox_unified: &[super::priority::IceboxItem],
    marker_id: u64,
) -> Vec<u64> {
    let Some(marker_idx) = icebox_unified.iter().position(|item| {
        matches!(
            item,
            super::priority::IceboxItem::Marker { id } if *id == marker_id
        )
    }) else {
        return Vec::new();
    };
    let group_start = icebox_unified[..marker_idx]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, item)| matches!(item, super::priority::IceboxItem::Marker { .. }))
        .map(|(i, _)| i + 1)
        .unwrap_or(0);
    icebox_unified[group_start..marker_idx]
        .iter()
        .filter_map(|item| match item {
            super::priority::IceboxItem::Story { number } => Some(*number),
            super::priority::IceboxItem::Marker { .. } => None,
        })
        .collect()
}

/// #1288: restore story-fold-positions adjacent to markers when the
/// stories are absent from icebox_unified post-restart. Truth-source
/// is each marker's persisted `stories` field (set by
/// `sync_marker_fold_snapshots` at the most-recent fold-mutation).
///
/// For each marker M in `store` with M.stories non-empty:
/// - If M is absent from icebox_unified, skip (marker itself must be
///   restored first by #1283's `restore_missing_markers_to_icebox_unified`)
/// - For each story-id in M.stories, in fold-order (top-of-fold
///   first), insert into icebox_unified IMMEDIATELY BEFORE M if not
///   already present. This preserves fold-order naturally: each new
///   insert pushes M one slot right, and subsequent inserts land just
///   before the new M-position, maintaining input-order.
/// - Stories whose canonical workflow_state is NOT Icebox should be
///   filtered out by the caller before this fires (e.g., via #1268's
///   `refresh_all` snapshot) — this function trusts the input list.
///
/// Returns the number of stories restored. Idempotent: re-run on
/// already-restored state finds stories present, inserts nothing.
pub fn restore_story_fold_positions_to_icebox_unified(
    store: &IterationMarkersStore,
    icebox_unified: &mut Vec<super::priority::IceboxItem>,
    canonical_icebox_stories: &std::collections::HashSet<u64>,
) -> usize {
    let mut restored = 0;
    for marker in &store.markers {
        if marker.stories.is_empty() {
            continue;
        }
        for story_id in &marker.stories {
            if !canonical_icebox_stories.contains(story_id) {
                continue;
            }
            let already_present = icebox_unified.iter().any(|item| {
                matches!(
                    item,
                    super::priority::IceboxItem::Story { number } if *number == *story_id
                )
            });
            if already_present {
                continue;
            }
            let Some(marker_idx) = icebox_unified.iter().position(|item| {
                matches!(
                    item,
                    super::priority::IceboxItem::Marker { id } if *id == marker.id
                )
            }) else {
                continue;
            };
            icebox_unified.insert(
                marker_idx,
                super::priority::IceboxItem::Story { number: *story_id },
            );
            restored += 1;
        }
    }
    restored
}

/// #1283: restore markers from the markers-store into `icebox_unified`
/// when they're missing from it post-restart. Reverse-direction sister
/// of `auto_heal_orphans` (#1271): that one evicts store-orphans
/// against icebox_unified-as-truth; this one treats markers-store as
/// evidence-that-marker-should-exist and ensures icebox_unified
/// contains it. Empirically observed substrate-divergence (Ryan
/// 2026-05-11 02:56 UTC): priority.json's icebox_unified is
/// effectively empty (Vec field, skip_serializing_if = Vec::is_empty
/// → omitted from JSON when state-mutation paths save a stale
/// snapshot) while markers-store retains 3 markers.
///
/// Truth-source for "marker exists" is markers-store. Markers missing
/// from icebox_unified are appended at the END in marker-position
/// order. Exact icebox_unified-position preservation requires a
/// schema-add (`icebox_position`) — deferred to follow-up; the
/// substrate-resilience user-job here is "marker remains in icebox
/// post-restart". Returns the number of markers restored.
///
/// Ordering: MUST run BEFORE `auto_heal_orphans` so #1271 doesn't
/// evict markers that THIS function is restoring (per #1283 AC 1).
/// Idempotent: if every store-marker is already in icebox_unified,
/// no-op.
pub fn restore_missing_markers_to_icebox_unified(
    store: &IterationMarkersStore,
    icebox_unified: &mut Vec<super::priority::IceboxItem>,
) -> usize {
    let present: std::collections::HashSet<u64> = icebox_unified
        .iter()
        .filter_map(|item| match item {
            super::priority::IceboxItem::Marker { id } => Some(*id),
            super::priority::IceboxItem::Story { .. } => None,
        })
        .collect();
    let mut sorted: Vec<&IterationMarker> = store
        .markers
        .iter()
        .filter(|m| !present.contains(&m.id))
        .collect();
    sorted.sort_by_key(|m| m.position);
    let restored = sorted.len();
    for m in sorted {
        icebox_unified.push(super::priority::IceboxItem::Marker { id: m.id });
    }
    restored
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
