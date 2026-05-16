//! Story priority ordering (#194).
//!
//! Persists at `~/.bpmux/priority.json`. Maps state labels to ordered
//! lists of issue numbers. PM sets the order; the board reads it.
//! Stories not in the file appear at the bottom of their column.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum IceboxItem {
    Story { number: u64 },
    Iteration { number: u64 },
}

fn deserialize_icebox_unified_tolerant<'de, D>(d: D) -> Result<Vec<IceboxItem>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Vec<serde_json::Value> = Vec::deserialize(d)?;
    Ok(raw
        .into_iter()
        .filter_map(|v| match v.get("kind").and_then(|k| k.as_str()) {
            Some("story") => v
                .get("number")
                .and_then(|n| n.as_u64())
                .map(|number| IceboxItem::Story { number }),
            Some("iteration") => v
                .get("number")
                .and_then(|n| n.as_u64())
                .map(|number| IceboxItem::Iteration { number }),
            _ => None,
        })
        .collect())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndex {
    #[serde(flatten)]
    pub columns: HashMap<String, Vec<u64>>,
    #[serde(
        default,
        rename = "icebox_unified",
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_icebox_unified_tolerant"
    )]
    pub icebox_unified: Vec<IceboxItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub current_iteration_stories: Vec<u64>,
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

/// #1268 backward-resilience boot-time auto-heal. Story entries in
/// `icebox_unified` whose canonical `workflow_state` is NOT `Icebox`
/// represent state-divergence — stories absent from `canonical_states`
/// are preserved (treated as unknown rather than stale).
pub fn auto_heal_icebox_unified_via_workflow_state(
    index: &mut PriorityIndex,
    canonical_states: &std::collections::HashMap<u64, super::super::board::domain::StoryState>,
) -> usize {
    let before = index.icebox_unified.len();
    index.icebox_unified.retain(|item| match item {
        IceboxItem::Story { number } => match canonical_states.get(number) {
            Some(super::super::board::domain::StoryState::Icebox) => true,
            Some(_) => false,
            None => true,
        },
        IceboxItem::Iteration { .. } => true,
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

/// #1301 Cassiopeia migration substrate: bring `icebox_unified` into
/// alignment with the canonical-iteration set on the running store.
///
/// Two operations in one pass:
///   1. Drop Iteration entries whose numbers no longer correspond to
///      a canonical Iteration story (stale test artifacts, deleted
///      iterations, etc.).
///   2. Append Iteration entries for canonical iterations not yet in
///      `icebox_unified`. New entries land at the bottom in
///      canonical-number order — PM can then drag them to their
///      preferred positions per Slice B's drag-reshape canon.
///
/// Idempotent: re-runs after the first pass are no-ops once the two
/// sets are in alignment. Returns the audit-summary so callers can
/// log what happened.
pub fn migrate_existing_iterations_into_icebox_unified(
    index: &mut PriorityIndex,
    canonical_iteration_numbers: &[u64],
) -> CassiopeiaMigrationAudit {
    let canonical: std::collections::HashSet<u64> =
        canonical_iteration_numbers.iter().copied().collect();
    let mut dropped: Vec<u64> = Vec::new();
    index.icebox_unified.retain(|item| match item {
        IceboxItem::Iteration { number } => {
            if canonical.contains(number) {
                true
            } else {
                dropped.push(*number);
                false
            }
        }
        IceboxItem::Story { .. } => true,
    });
    let present: std::collections::HashSet<u64> = index
        .icebox_unified
        .iter()
        .filter_map(|item| match item {
            IceboxItem::Iteration { number } => Some(*number),
            IceboxItem::Story { .. } => None,
        })
        .collect();
    let mut to_insert: Vec<u64> = canonical_iteration_numbers
        .iter()
        .copied()
        .filter(|n| !present.contains(n))
        .collect();
    to_insert.sort_unstable();
    for n in to_insert.iter().copied() {
        index
            .icebox_unified
            .push(IceboxItem::Iteration { number: n });
    }
    CassiopeiaMigrationAudit {
        dropped_stale: dropped,
        inserted_missing: to_insert,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CassiopeiaMigrationAudit {
    pub dropped_stale: Vec<u64>,
    pub inserted_missing: Vec<u64>,
}

impl CassiopeiaMigrationAudit {
    pub fn is_noop(&self) -> bool {
        self.dropped_stale.is_empty() && self.inserted_missing.is_empty()
    }
}

/// #1258 α-substrate migration: populate `icebox_unified` from legacy
/// `columns["icebox"]` story-list on first server boot post-α-ship.
/// Idempotent: if `icebox_unified` is already populated, returns false
/// without touching state.
pub fn migrate_icebox_unified(index: &mut PriorityIndex) -> bool {
    if !index.icebox_unified.is_empty() {
        return false;
    }
    let stories: Vec<u64> = index.columns.get("icebox").cloned().unwrap_or_default();
    if stories.is_empty() {
        return false;
    }
    for n in stories {
        index.icebox_unified.push(IceboxItem::Story { number: n });
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
