//! Story priority ordering (#194).
//!
//! Persists at `~/.bpmux/priority.json`. Maps state labels to ordered
//! lists of issue numbers. PM sets the order; the board reads it.
//! Stories not in the file appear at the bottom of their column.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum IceboxItem {
    Story { number: u64 },
    Marker { id: u64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndex {
    #[serde(flatten)]
    pub columns: HashMap<String, Vec<u64>>,
    #[serde(
        default,
        rename = "icebox_unified",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub icebox_unified: Vec<IceboxItem>,
}

/// Default priority file path: `~/.bpmux/priority.json`.
pub fn default_priority_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "cannot determine home directory".to_string())?;
    Ok(home.join(".bpmux").join("priority.json"))
}

pub fn load_priority(path: &Path) -> PriorityIndex {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => PriorityIndex::default(),
    }
}

/// #1258 close_iteration outcome — the cross-store atomic step that
/// (a) promotes stories above the topmost iteration marker into the
/// ready queue, (b) deletes the topmost marker, (c) drops unfinished
/// ready-stories back to the top of icebox_unified.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseIterationOutcome {
    pub promoted: Vec<u64>,
    pub dropped_back: Vec<u64>,
    pub deleted_marker_id: Option<u64>,
}

/// #1258 close_iteration α-substrate atomic step. Idempotent only in
/// the no-markers case — when markers exist, calling this consumes
/// the topmost marker and shifts state. Returns the outcome for
/// caller-side WS broadcasts.
///
/// Semantic:
/// 1. If no Marker exists in `icebox_unified`, returns empty Outcome
///    (no-op; caller should fall back to legacy signal-only behavior).
/// 2. Otherwise: promote = Story items in `icebox_unified[0..k]` where
///    k is the index of the first Marker. These are appended to
///    `priority.columns["ready"]`.
/// 3. Delete = the marker at index k. `icebox_unified[0..=k]` are
///    removed; remaining items shift up.
/// 4. Drop-back = `unfinished_ready` (caller-supplied: ready-state
///    story numbers that weren't promoted from prior iterations).
///    These prepend to `icebox_unified` as Story items, and are
///    removed from `priority.columns["ready"]`.
/// 5. Markers list: the deleted marker is removed from `markers`.
pub fn close_iteration_atomic(
    priority: &mut PriorityIndex,
    markers: &mut Vec<super::iteration_markers::IterationMarker>,
    unfinished_ready: &[u64],
    carry_over: bool,
) -> CloseIterationOutcome {
    let topmost = priority
        .icebox_unified
        .iter()
        .enumerate()
        .find_map(|(i, item)| match item {
            IceboxItem::Marker { id } => Some((i, *id)),
            _ => None,
        });
    let Some((k, deleted_marker_id)) = topmost else {
        return CloseIterationOutcome::default();
    };
    let promoted: Vec<u64> = priority.icebox_unified[..k]
        .iter()
        .filter_map(|item| match item {
            IceboxItem::Story { number } => Some(*number),
            _ => None,
        })
        .collect();
    priority.icebox_unified.drain(0..=k);
    let dropped_back: Vec<u64> = if carry_over {
        Vec::new()
    } else {
        unfinished_ready.to_vec()
    };
    for &n in dropped_back.iter().rev() {
        priority
            .icebox_unified
            .insert(0, IceboxItem::Story { number: n });
    }
    if let Some(ready) = priority.columns.get_mut("ready") {
        ready.retain(|n| !dropped_back.contains(n));
    }
    let ready = priority.columns.entry("ready".to_string()).or_default();
    for n in &promoted {
        ready.push(*n);
    }
    markers.retain(|m| m.id != deleted_marker_id);
    CloseIterationOutcome {
        promoted,
        dropped_back,
        deleted_marker_id: Some(deleted_marker_id),
    }
}

/// #1258 AC 22 testable persistence wrapper around `close_iteration_atomic`.
/// Calls the atomic function, then save_priority, then save_markers; if
/// save_markers fails, rolls priority back to `priority_snapshot` via a
/// second save_priority call. Returns the Outcome on success, or an error
/// string on save-failure (after rollback attempt). This shape lets tests
/// inject save-failure without touching disk and assert the rollback path
/// is exercised.
pub fn apply_close_iteration_with_rollback<SaveP, SaveM>(
    priority: &mut PriorityIndex,
    markers: &mut Vec<super::iteration_markers::IterationMarker>,
    unfinished_ready: &[u64],
    carry_over: bool,
    priority_snapshot: &PriorityIndex,
    save_priority: SaveP,
    save_markers: SaveM,
) -> Result<CloseIterationOutcome, String>
where
    SaveP: Fn(&PriorityIndex) -> Result<(), String>,
    SaveM: FnOnce(&[super::iteration_markers::IterationMarker]) -> Result<(), String>,
{
    let outcome = close_iteration_atomic(priority, markers, unfinished_ready, carry_over);
    save_priority(priority).map_err(|e| format!("save priority: {e}"))?;
    if let Err(e) = save_markers(markers) {
        if let Err(rb) = save_priority(priority_snapshot) {
            return Err(format!(
                "markers save failed: {e}; rollback also failed: {rb}"
            ));
        }
        return Err(format!("markers save failed (priority rolled back): {e}"));
    }
    Ok(outcome)
}

/// #1258 α-substrate migration: populate `icebox_unified` from legacy
/// state (separate `columns["icebox"]` story-list + per-session
/// iteration markers) on first server boot post-α-ship. Idempotent:
/// if `icebox_unified` is already populated, returns false without
/// touching state. Default ordering: legacy icebox stories (priority
/// order) first, then iteration markers (position order). Legacy
/// state had no story↔marker grouping, so markers post-migration
/// represent "next iteration boundaries awaiting assignment" — owner
/// can drag-reorder to interleave.
pub fn migrate_icebox_unified(
    index: &mut PriorityIndex,
    markers: &[super::iteration_markers::IterationMarker],
) -> bool {
    if !index.icebox_unified.is_empty() {
        return false;
    }
    let stories: Vec<u64> = index.columns.get("icebox").cloned().unwrap_or_default();
    let mut sorted_markers: Vec<&super::iteration_markers::IterationMarker> =
        markers.iter().collect();
    sorted_markers.sort_by_key(|m| m.position);
    if stories.is_empty() && sorted_markers.is_empty() {
        return false;
    }
    for n in stories {
        index.icebox_unified.push(IceboxItem::Story { number: n });
    }
    for m in sorted_markers {
        index.icebox_unified.push(IceboxItem::Marker { id: m.id });
    }
    true
}

pub fn save_priority(path: &Path, index: &PriorityIndex) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::other(format!("serialize: {e}")))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Move a story to a specific position (1-indexed) within its state
/// column. If the story isn't in the column yet, it's inserted.
pub fn reorder(index: &mut PriorityIndex, state: &str, number: u64, position: usize) {
    let column = index.columns.entry(state.to_string()).or_default();
    column.retain(|&n| n != number);
    let pos = position.saturating_sub(1).min(column.len());
    column.insert(pos, number);
}

/// #1258 α-substrate: move a story or marker to a specific position
/// (1-indexed) within `icebox_unified`. If the item isn't in the
/// list yet, it's inserted; otherwise it's moved. Position is 1-
/// indexed across all IceboxItems (stories AND markers share the
/// positional space).
pub fn reorder_icebox_unified(index: &mut PriorityIndex, item: IceboxItem, position: usize) {
    index.icebox_unified.retain(|x| x != &item);
    let pos = position.saturating_sub(1).min(index.icebox_unified.len());
    index.icebox_unified.insert(pos, item);
}

/// Sort a list of issue numbers by priority. Numbers in the priority
/// list come first (in priority order); others are appended at the end
/// in their original order.
pub fn sort_by_priority(numbers: &[u64], priority: &[u64]) -> Vec<u64> {
    let mut ordered: Vec<u64> = Vec::new();
    for &n in priority {
        if numbers.contains(&n) {
            ordered.push(n);
        }
    }
    for &n in numbers {
        if !ordered.contains(&n) {
            ordered.push(n);
        }
    }
    ordered
}

/// #333: apply the priority index to a board-stories array (as returned
/// by `tracker.fetch_board_stories`). Stories are grouped by their
/// `state` field in encounter order; within each state, order follows
/// the priority index (missing entries fall to the end in original
/// order). Without this, drag-to-reorder on the frontend persists via
/// POST /api/stories/:n/reorder but the very next /api/board refetch
/// returns GitHub's native order, snapping the UI back.
pub fn apply_priority_to_board(
    stories: Vec<serde_json::Value>,
    priority: &PriorityIndex,
) -> Vec<serde_json::Value> {
    let mut by_state: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    for s in stories {
        let state = s
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        match by_state.iter_mut().find(|(k, _)| k == &state) {
            Some((_, list)) => list.push(s),
            None => by_state.push((state, vec![s])),
        }
    }
    for (state, list) in by_state.iter_mut() {
        if let Some(pri) = priority.columns.get(state) {
            list.sort_by_cached_key(|s| {
                let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                pri.iter().position(|&p| p == number).unwrap_or(usize::MAX)
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "priority_tests.rs"]
mod tests;
