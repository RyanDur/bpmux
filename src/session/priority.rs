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

#[cfg(test)]
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
}
