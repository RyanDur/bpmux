use crate::product_management::story::{BoardState, Stage, Story};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

entity_id!(EpicId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epic {
    pub id: EpicId,
    pub title: String,
    pub body: Option<String>,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub state: BoardState,
    pub is_closed: bool,
}

#[derive(Default, Clone)]
pub struct EpicState {
    pub epics: Vec<Epic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpicAction {
    Seeded(Vec<Epic>),
    Upserted(Vec<Epic>),
}

pub fn reduce(state: &EpicState, action: &EpicAction) -> Option<EpicState> {
    match action {
        EpicAction::Seeded(epics) => {
            if state.epics == *epics {
                None
            } else {
                Some(EpicState {
                    epics: epics.clone(),
                })
            }
        }
        EpicAction::Upserted(incoming) => {
            let mut epics = state.epics.clone();
            for item in incoming {
                match epics.iter().position(|existing| existing.id == item.id) {
                    Some(index) => epics[index] = item.clone(),
                    None => epics.push(item.clone()),
                }
            }
            if epics == state.epics {
                None
            } else {
                Some(EpicState { epics })
            }
        }
    }
}

pub fn all(state: &EpicState) -> Vec<Epic> {
    state.epics.clone()
}

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
            BoardState::Accepted => counts.accepted_count += 1,
            BoardState::Retired => counts.retired_count += 1,
            BoardState::Icebox => counts.icebox_count += 1,
            BoardState::Ready | BoardState::InProgress(_) | BoardState::Acceptance => {
                counts.active_count += 1
            }
        }
    }
    counts
}

pub fn derive_epic_state(_epic_explicit_state: BoardState, children: &[&Story]) -> BoardState {
    if children.is_empty() || children.iter().all(|c| c.state() == BoardState::Icebox) {
        return BoardState::Icebox;
    }
    let any_active = children.iter().any(|c| {
        matches!(
            c.state(),
            BoardState::InProgress(_) | BoardState::Acceptance
        )
    });
    if any_active {
        return BoardState::InProgress(Stage::Dev.id());
    }
    let non_retired: Vec<&&Story> = children
        .iter()
        .filter(|c| c.state() != BoardState::Retired)
        .collect();
    if non_retired.is_empty() {
        return BoardState::Retired;
    }
    if non_retired
        .iter()
        .all(|c| c.state() == BoardState::Accepted)
    {
        return BoardState::Accepted;
    }
    if children.iter().any(|c| c.state() == BoardState::Ready) {
        return BoardState::Ready;
    }
    BoardState::Icebox
}
