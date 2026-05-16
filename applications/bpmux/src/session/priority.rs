//! Legacy priority.json shim (#1301 slice 4E).
//!
//! Pre-Cassiopeia, this module owned per-state-column positional
//! ordering at `~/.bpmux/priority.json`. After slice 4E all
//! authoritative ordering lives in canonical-via-GH labels:
//! `iteration-{Theme}.{N}` for icebox iteration membership and
//! `ready-order.{N}` for the ready queue.
//!
//! What survives:
//!   - [`PriorityIndex`] + `load_priority` + `save_priority`: still
//!     used by [`crate::session::storage::Storage`] so the storage
//!     trait surface stays stable for in-memory test harnesses; the
//!     field is effectively dead-store now (no writers).
//!   - [`default_priority_path`]: still queried by the boot migration
//!     [`crate::migrate_legacy_ready_to_labels`] for one-shot legacy
//!     readout.
//!   - [`sort_board_by_ready_order_labels`]: pure label-based sorter
//!     for the `/api/board` response; replaces the prior
//!     `apply_priority_to_board`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndex {
    #[serde(flatten)]
    pub columns: HashMap<String, Vec<u64>>,
}

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

/// Sort a board-stories array by `ready-order.{N}` labels for the
/// `ready` group. Other groups pass through in encounter order.
///
/// Ready ordering: stories carrying a `ready-order.{N}` label sort
/// ascending by label position. Unlabeled stories follow, ordered by
/// issue number for stability (transitional shape until boot
/// migration has labeled every legacy entry — see [`crate::migrate_legacy_ready_to_labels`]).
pub fn sort_board_by_ready_order_labels(stories: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
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
        if state == "ready" {
            list.sort_by_cached_key(|s| {
                let label_pos = ready_order_from_value(s).map(u64::from);
                let number = s.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                (label_pos.unwrap_or(u64::MAX), number)
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

fn ready_order_from_value(s: &serde_json::Value) -> Option<u32> {
    s.get("labels")?.as_array()?.iter().find_map(|l| {
        let name = l.as_str().or_else(|| l.get("name")?.as_str())?;
        let rest = name.strip_prefix("ready-order.")?;
        rest.parse::<u32>().ok()
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "priority_tests.rs"]
mod tests;
