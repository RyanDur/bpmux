//! Story priority ordering (#194).
//!
//! Persists at `~/.bpmux/priority.json`. Maps state labels to ordered
//! lists of issue numbers. PM sets the order; the board reads it.
//! Stories not in the file appear at the bottom of their column.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Ordered list of issue numbers per state column.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndex {
    /// Key: state label (lowercase, e.g. "ready", "in_progress").
    /// Value: ordered list of issue numbers, highest priority first.
    #[serde(flatten)]
    pub columns: HashMap<String, Vec<u64>>,
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
    // Remove from current position if present
    column.retain(|&n| n != number);
    // Insert at target position (1-indexed, clamped)
    let pos = position.saturating_sub(1).min(column.len());
    column.insert(pos, number);
}

/// Sort a list of issue numbers by priority. Numbers in the priority
/// list come first (in priority order); others are appended at the end
/// in their original order.
pub fn sort_by_priority(numbers: &[u64], priority: &[u64]) -> Vec<u64> {
    let mut ordered: Vec<u64> = Vec::new();
    // First: items from priority list that exist in numbers
    for &n in priority {
        if numbers.contains(&n) {
            ordered.push(n);
        }
    }
    // Then: items not in priority list, in original order
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
    // Preserve inter-state encounter order via a Vec of (state, list);
    // a HashMap would reorder states alphabetically which is fine for
    // the frontend (re-groups by state) but harder to reason about in
    // tests.
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
mod tests {
    use super::*;

    #[test]
    fn reorder_inserts_at_position() {
        let mut index = PriorityIndex::default();
        reorder(&mut index, "ready", 100, 1);
        reorder(&mut index, "ready", 200, 1);
        // 200 should be first, 100 second
        assert_eq!(index.columns["ready"], vec![200, 100]);
    }

    #[test]
    fn reorder_moves_existing_story() {
        let mut index = PriorityIndex::default();
        reorder(&mut index, "ready", 100, 1);
        reorder(&mut index, "ready", 200, 2);
        reorder(&mut index, "ready", 300, 3);
        // Move 300 to position 1
        reorder(&mut index, "ready", 300, 1);
        assert_eq!(index.columns["ready"], vec![300, 100, 200]);
    }

    #[test]
    fn reorder_clamps_position() {
        let mut index = PriorityIndex::default();
        reorder(&mut index, "ready", 100, 1);
        // Position 99 should clamp to end
        reorder(&mut index, "ready", 200, 99);
        assert_eq!(index.columns["ready"], vec![100, 200]);
    }

    #[test]
    fn sort_by_priority_orders_correctly() {
        let numbers = vec![100, 200, 300, 400];
        let priority = vec![300, 100];
        let sorted = sort_by_priority(&numbers, &priority);
        assert_eq!(sorted, vec![300, 100, 200, 400]);
    }

    #[test]
    fn sort_by_priority_handles_empty_priority() {
        let numbers = vec![100, 200, 300];
        let sorted = sort_by_priority(&numbers, &[]);
        assert_eq!(sorted, vec![100, 200, 300]);
    }

    #[test]
    fn save_and_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("priority.json");
        let mut index = PriorityIndex::default();
        reorder(&mut index, "ready", 100, 1);
        reorder(&mut index, "ready", 200, 2);
        save_priority(&path, &index).unwrap();
        let loaded = load_priority(&path);
        assert_eq!(loaded.columns["ready"], vec![100, 200]);
    }

    // ── #333 apply_priority_to_board tests ──

    fn story(number: u64, state: &str) -> serde_json::Value {
        serde_json::json!({ "number": number, "state": state, "title": "s" })
    }

    fn numbers_of(stories: &[serde_json::Value]) -> Vec<u64> {
        stories
            .iter()
            .filter_map(|s| s.get("number").and_then(|v| v.as_u64()))
            .collect()
    }

    #[test]
    fn apply_priority_reorders_within_ready_column() {
        // Baseline falsifier: priority [300, 100] within ready must
        // reorder [100, 200, 300] → [300, 100, 200]. Without this
        // fix, the frontend's drag-reorder would snap back after the
        // first /api/board refetch.
        let mut index = PriorityIndex::default();
        index.columns.insert("ready".to_string(), vec![300, 100]);
        let stories = vec![
            story(100, "ready"),
            story(200, "ready"),
            story(300, "ready"),
        ];
        let sorted = apply_priority_to_board(stories, &index);
        assert_eq!(numbers_of(&sorted), vec![300, 100, 200]);
    }

    #[test]
    fn apply_priority_leaves_other_states_untouched() {
        // Only the column with a priority entry sorts; other columns
        // pass through in their original order.
        let mut index = PriorityIndex::default();
        index.columns.insert("ready".to_string(), vec![200, 100]);
        let stories = vec![
            story(100, "ready"),
            story(200, "ready"),
            story(500, "delivered"),
            story(400, "delivered"),
        ];
        let sorted = apply_priority_to_board(stories, &index);
        assert_eq!(numbers_of(&sorted), vec![200, 100, 500, 400]);
    }

    #[test]
    fn apply_priority_preserves_inter_state_encounter_order() {
        // States appear in the output in the order they first appear
        // in the input. Frontend re-groups by state anyway, but
        // stable ordering makes the function easier to reason about.
        let mut index = PriorityIndex::default();
        index.columns.insert("ready".to_string(), vec![100]);
        let stories = vec![
            story(500, "delivered"),
            story(100, "ready"),
            story(600, "delivered"),
        ];
        let sorted = apply_priority_to_board(stories, &index);
        // Delivered appears first (encounter order), then ready; within
        // delivered, original order preserved; ready has only one item.
        assert_eq!(numbers_of(&sorted), vec![500, 600, 100]);
    }

    #[test]
    fn apply_priority_with_empty_index_is_identity() {
        // No priority entries anywhere → input order preserved.
        let index = PriorityIndex::default();
        let stories = vec![
            story(100, "ready"),
            story(200, "ready"),
            story(300, "ready"),
        ];
        let sorted = apply_priority_to_board(stories.clone(), &index);
        assert_eq!(numbers_of(&sorted), numbers_of(&stories));
    }

    #[test]
    fn apply_priority_entries_for_absent_numbers_ignored() {
        // Priority list references story 999 which isn't in the
        // current fetch — that entry is ignored; remaining priority
        // entries still reorder the present stories.
        let mut index = PriorityIndex::default();
        index
            .columns
            .insert("ready".to_string(), vec![999, 200, 100]);
        let stories = vec![
            story(100, "ready"),
            story(200, "ready"),
            story(300, "ready"),
        ];
        let sorted = apply_priority_to_board(stories, &index);
        assert_eq!(numbers_of(&sorted), vec![200, 100, 300]);
    }
}
