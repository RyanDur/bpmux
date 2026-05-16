//! Story priority ordering (#194).
//!
//! Persists at `~/.bpmux/priority.json`. Maps state labels to ordered
//! lists of issue numbers. PM sets the order; the board reads it.
//! Stories not in the file appear at the bottom of their column.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndex {
    #[serde(flatten)]
    pub columns: HashMap<String, Vec<u64>>,
}

/// Default priority file path. Honors `BPMUX_PRIORITY_PATH` for per-test
/// isolation (sister to BPMUX_USAGE_PATH / BPMUX_AUTH_TOKEN_PATH /
/// BPMUX_SESSIONS_BASE — the established isolation pattern at this
/// substrate layer). Falls back to `~/.bpmux/priority.json` in prod.
/// Without env-isolation, parallel `cargo test` processes share the
/// real user's priority.json and race on the create_dir_all + write +
/// rename sequence inside `save_priority`, surfacing as transient
/// `No such file or directory` failures (deterministic under load).
pub fn default_priority_path() -> Result<PathBuf, String> {
    if let Ok(override_path) = std::env::var("BPMUX_PRIORITY_PATH")
        && !override_path.is_empty()
    {
        return Ok(PathBuf::from(override_path));
    }
    let home = dirs::home_dir().ok_or_else(|| "cannot determine home directory".to_string())?;
    Ok(home.join(".bpmux").join("priority.json"))
}

pub fn load_priority(path: &Path) -> PriorityIndex {
    load_priority_raw(path)
}

pub fn load_priority_raw(path: &Path) -> PriorityIndex {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => PriorityIndex::default(),
    }
}

/// #1272 backward-resilience boot-time auto-heal at ready-column-axis.
/// Legacy `columns["ready"]` array entries whose canonical
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
/// than stale — safer default).
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
