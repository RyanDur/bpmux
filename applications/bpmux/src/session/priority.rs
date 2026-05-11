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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub current_iteration_stories: Vec<u64>,
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

/// #1258 close_iteration outcome — promotes stories above the topmost
/// iteration marker into the ready queue and deletes the topmost marker.
/// Post-#1258 r3 (Ryan 21:49 UTC guard-directive): unfinished-ready
/// drop-back is removed; the caller-side guard ensures the ready queue
/// is empty before this fires.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseIterationOutcome {
    pub promoted: Vec<u64>,
    pub deleted_marker_id: Option<u64>,
}

pub fn close_iteration_atomic(
    priority: &mut PriorityIndex,
    markers: &mut Vec<super::iteration_markers::IterationMarker>,
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
    let ready = priority.columns.entry("ready".to_string()).or_default();
    // #1272: dedup before append. Pre-#1272, repeated close_iteration
    // fires accumulated duplicate entries in columns["ready"] (e.g.,
    // priority.json on 2026-05-10 22:50 UTC showed #1251-#1254 each
    // appearing twice). Remove-then-append per story-number is the
    // idempotent shape.
    for n in &promoted {
        ready.retain(|x| x != n);
        ready.push(*n);
    }
    markers.retain(|m| m.id != deleted_marker_id);
    priority.current_iteration_stories = promoted.clone();
    CloseIterationOutcome {
        promoted,
        deleted_marker_id: Some(deleted_marker_id),
    }
}

pub fn apply_close_iteration_with_rollback<SaveP, SaveM>(
    priority: &mut PriorityIndex,
    markers: &mut Vec<super::iteration_markers::IterationMarker>,
    priority_snapshot: &PriorityIndex,
    save_priority: SaveP,
    save_markers: SaveM,
) -> Result<CloseIterationOutcome, String>
where
    SaveP: Fn(&PriorityIndex) -> Result<(), String>,
    SaveM: FnOnce(&[super::iteration_markers::IterationMarker]) -> Result<(), String>,
{
    let outcome = close_iteration_atomic(priority, markers);
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

/// #1268 backward-resilience boot-time auto-heal. Story entries in
/// `icebox_unified` whose canonical `workflow_state` is NOT `Icebox`
/// represent state-divergence — substrate regression, pre-#1266
/// stale state, or rollback to older binaries left them behind.
/// Truth-source is canonical workflow_state per story (NOT legacy
/// `columns["icebox"]`, which #1261's `apply_reorder_story` actively
/// empties for icebox-resident stories). Sister-class to #1266
/// forward cascade-clean at boot-axis. Returns count of evicted
/// Story entries. Marker entries preserved unconditionally.
/// Stories absent from `canonical_states` are preserved (treated as
/// unknown rather than stale — safer default).
pub fn auto_heal_icebox_unified_via_workflow_state(
    index: &mut PriorityIndex,
    canonical_states: &std::collections::HashMap<u64, super::super::board::domain::StoryState>,
) -> usize {
    let before = index.icebox_unified.len();
    index.icebox_unified.retain(|item| match item {
        IceboxItem::Marker { .. } => true,
        IceboxItem::Story { number } => match canonical_states.get(number) {
            Some(super::super::board::domain::StoryState::Icebox) => true,
            Some(_) => false,
            None => true,
        },
    });
    before - index.icebox_unified.len()
}

/// #1272 backward-resilience boot-time auto-heal at ready-column-axis.
/// Sister-pattern to `auto_heal_icebox_unified_via_workflow_state` at
/// icebox-axis: legacy `columns["ready"]` array entries whose canonical
/// `workflow_state` is NOT `Ready` represent state-divergence —
/// missing forward-cascade-clean on state-transition out of ready
/// (move_story / accept_story / close_story / etc.) OR pre-#1272
/// stale state OR rollback to older binaries.
///
/// Empirically observed (PM 2026-05-10 22:50 UTC ground-check):
/// priority.json `ready` array = [100, 1269, 1268, 1251, 1252, 1253,
/// 1254, 1251, 1252, 1253, 1254] — accepted-state stories (#100, #1268,
/// #1269) stuck in legacy ready array + duplicates from repeated
/// close_iteration fires. Canonical `list_ready` correctly returned
/// `[]`.
///
/// Truth-source = canonical workflow_state per story. Stories absent
/// from `canonical_states` are preserved (treated as unknown rather
/// than stale — safer default; sister-shape to icebox heal).
/// Deduplicates in-pass: if the same number appears multiple times,
/// only the first occurrence with workflow_state=Ready survives.
/// Returns count of evicted entries.
pub fn auto_heal_ready_column_via_workflow_state(
    index: &mut PriorityIndex,
    canonical_states: &std::collections::HashMap<u64, super::super::board::domain::StoryState>,
) -> usize {
    let Some(ready) = index.columns.get_mut("ready") else {
        return 0;
    };
    let before = ready.len();
    let mut seen = std::collections::HashSet::new();
    ready.retain(|number| {
        if !seen.insert(*number) {
            return false;
        }
        match canonical_states.get(number) {
            Some(super::super::board::domain::StoryState::Ready) => true,
            Some(_) => false,
            None => true,
        }
    });
    before - ready.len()
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
