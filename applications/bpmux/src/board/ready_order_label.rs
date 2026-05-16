//! Symmetric Rust counterpart to ready-column ordering via GH labels.
//!
//! `ready-order.{position}` labels are the canonical-via-GH substrate
//! for the ready queue's user-visible ordering (#1301 slice 4B+).
//! Sister to [`super::iteration_label`] which handles iteration
//! membership ordering via `iteration-{Theme}.{position}` labels.
//!
//! Format pinned: `ready-order.<u32-position>` — sparse-numeric
//! algebra (default step 100) so midpoint insertions don't cascade-
//! rewrite every neighbor on each reorder.

use crate::board::domain::{Story, StoryState};

pub fn parse_ready_order_label(label: &str) -> Option<u32> {
    let rest = label.strip_prefix("ready-order.")?;
    rest.parse::<u32>().ok()
}

pub fn is_ready_order_label(label: &str) -> bool {
    parse_ready_order_label(label).is_some()
}

pub fn story_ready_order_position(story: &Story) -> Option<u32> {
    story
        .raw_labels
        .iter()
        .filter_map(|l| parse_ready_order_label(l))
        .min()
}

pub fn stories_with_ready_order(stories: &[Story]) -> Vec<u64> {
    let mut ordered: Vec<(u64, u32)> = stories
        .iter()
        .filter(|s| !s.is_closed && s.state == StoryState::Ready)
        .filter_map(|s| story_ready_order_position(s).map(|pos| (s.number, pos)))
        .collect();
    ordered.sort_by_key(|(_, pos)| *pos);
    ordered.into_iter().map(|(n, _)| n).collect()
}

/// Sort a board-stories JSON array by `ready-order.{N}` labels for the
/// `ready` group. Other groups pass through in encounter order.
///
/// Ready ordering: stories carrying a `ready-order.{N}` label sort
/// ascending by label position. Unlabeled stories follow, ordered by
/// issue number for stability (transitional shape until boot
/// migration has labeled every legacy entry — see
/// [`crate::migrate_legacy_ready_to_labels`]).
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "ready_order_label_tests.rs"]
mod tests;
