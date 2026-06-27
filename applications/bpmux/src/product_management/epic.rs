use crate::product_management::story::{Story, StoryState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EpicCounts {
    pub accepted_count: usize,
    pub retired_count: usize,
    pub active_count: usize,
    pub icebox_count: usize,
    pub total_count: usize,
}

impl EpicCounts {
    pub fn progress_denominator(&self) -> usize {
        self.accepted_count + self.active_count + self.icebox_count
    }
}

pub fn derive_epic_counts(children: &[&Story]) -> EpicCounts {
    let mut counts = EpicCounts {
        accepted_count: 0,
        retired_count: 0,
        active_count: 0,
        icebox_count: 0,
        total_count: children.len(),
    };
    for child in children {
        match child.state() {
            StoryState::Accepted => counts.accepted_count += 1,
            StoryState::Retired => counts.retired_count += 1,
            StoryState::Icebox => counts.icebox_count += 1,
            StoryState::Ready
            | StoryState::InProgress
            | StoryState::DeskCheck
            | StoryState::Delivered
            | StoryState::Acceptance => counts.active_count += 1,
        }
    }
    counts
}

pub fn derive_epic_state(_epic_explicit_state: StoryState, children: &[&Story]) -> StoryState {
    if children.is_empty() || children.iter().all(|c| c.state() == StoryState::Icebox) {
        return StoryState::Icebox;
    }
    let any_active = children.iter().any(|c| {
        matches!(
            c.state(),
            StoryState::InProgress
                | StoryState::DeskCheck
                | StoryState::Delivered
                | StoryState::Acceptance
        )
    });
    if any_active {
        return StoryState::InProgress;
    }
    let non_retired: Vec<&&Story> = children
        .iter()
        .filter(|c| c.state() != StoryState::Retired)
        .collect();
    if non_retired.is_empty() {
        return StoryState::Retired;
    }
    if non_retired
        .iter()
        .all(|c| c.state() == StoryState::Accepted)
    {
        return StoryState::Accepted;
    }
    if children.iter().any(|c| c.state() == StoryState::Ready) {
        return StoryState::Ready;
    }
    StoryState::Icebox
}
