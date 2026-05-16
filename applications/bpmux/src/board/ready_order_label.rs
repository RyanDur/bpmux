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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "ready_order_label_tests.rs"]
mod tests;
