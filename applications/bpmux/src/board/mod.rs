use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use product_management::story::events::StoryEventKind;
use product_management::story::slice::StoryAction;
use product_management::story::{Stage, Story, StoryId, Work};
use workflow::StageId;

pub mod flow;
pub mod history;
pub mod history_persistence;
pub mod persistence;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod history_persistence_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod persistence_tests;
#[cfg(test)]
mod placement_tests;
#[cfg(test)]
mod verdict_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardAction {
    StoryPlaced { story: StoryId, stage: StageId },
    StoryUnplaced { story: StoryId },
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct BoardPlacements {
    placements: HashMap<StoryId, StageId>,
}

impl BoardPlacements {
    pub fn pairs(&self) -> Vec<(StoryId, StageId)> {
        self.placements
            .iter()
            .map(|(story, stage)| (*story, stage.clone()))
            .collect()
    }
}

pub fn reduce(state: &BoardPlacements, action: &BoardAction) -> Option<BoardPlacements> {
    match action {
        BoardAction::StoryPlaced { story, stage } => {
            if state.placements.get(story) == Some(stage) {
                return None;
            }
            let mut placements = state.placements.clone();
            placements.insert(*story, stage.clone());
            Some(BoardPlacements { placements })
        }
        BoardAction::StoryUnplaced { story } => {
            if !state.placements.contains_key(story) {
                return None;
            }
            let mut placements = state.placements.clone();
            placements.remove(story);
            Some(BoardPlacements { placements })
        }
    }
}

pub fn placement_of(state: &BoardPlacements, story: StoryId) -> Option<StageId> {
    state.placements.get(&story).cloned()
}

pub fn placed_story_ids(state: &BoardPlacements) -> Vec<StoryId> {
    state.placements.keys().copied().collect()
}

pub fn react_to_story(state: &BoardPlacements, action: &StoryAction) -> Option<BoardPlacements> {
    reduce(state, &placement_edge(state, action)?)
}

pub(crate) fn placement_edge(state: &BoardPlacements, action: &StoryAction) -> Option<BoardAction> {
    let (id, placement) = match action {
        StoryAction::Event(event) => match &event.kind {
            StoryEventKind::StateTransitioned { placement, .. } => (event.id, placement),
            _ => return None,
        },
        StoryAction::Events(events) => events.iter().rev().find_map(|event| match &event.kind {
            StoryEventKind::StateTransitioned { placement, .. } => Some((event.id, placement)),
            _ => None,
        })?,
        _ => return None,
    };
    match placement {
        Some(stage) if placement_of(state, id).as_ref() != Some(stage) => {
            Some(BoardAction::StoryPlaced {
                story: id,
                stage: stage.clone(),
            })
        }
        None if placement_of(state, id).is_some() => Some(BoardAction::StoryUnplaced { story: id }),
        _ => None,
    }
}

pub fn fine_stage(story: &Story, board: &BoardPlacements) -> Option<Stage> {
    match &story.state {
        Work::InProgress => Some(
            placement_of(board, story.id)
                .and_then(|stage_id| Stage::from_id(&stage_id))
                .unwrap_or(Stage::Dev),
        ),
        _ => None,
    }
}

pub fn held_stage(story: &Story, board: &BoardPlacements) -> Option<StageId> {
    match &story.state {
        Work::InProgress => Some(placement_of(board, story.id).unwrap_or_else(|| {
            workflow::board_middle_stage_ids(workflow::current_workflow())
                .first()
                .cloned()
                .unwrap_or_else(|| Stage::Dev.id())
        })),
        _ => None,
    }
}

pub fn placement_from_target(target: &str, middle: &[StageId]) -> Option<StageId> {
    if middle.first().is_some_and(|entry| entry.as_str() == target) {
        return None;
    }
    middle
        .iter()
        .find(|stage| stage.as_str() == target)
        .cloned()
}

pub fn placements_for_ingest(
    stories: &[Story],
    desk_check: &[(StoryId, StageId)],
) -> Vec<BoardAction> {
    let desk_check: HashMap<StoryId, &StageId> =
        desk_check.iter().map(|(id, stage)| (*id, stage)).collect();
    stories
        .iter()
        .filter_map(|story| match story.state {
            Work::InProgress => desk_check
                .get(&story.id)
                .map(|stage| BoardAction::StoryPlaced {
                    story: story.id,
                    stage: (*stage).clone(),
                }),
            _ => None,
        })
        .collect()
}

pub fn state_label(
    coarse: &product_management::scheduling::ProjectedState,
    fine: Option<Stage>,
) -> &'static str {
    match fine {
        Some(Stage::DeskCheck) => "desk-check",
        Some(Stage::Dev) => "in-progress",
        None => coarse.as_str(),
    }
}

pub fn wire_state(
    coarse: &product_management::scheduling::ProjectedState,
    held: Option<&StageId>,
) -> String {
    use product_management::scheduling::ProjectedState;
    match (coarse, held) {
        (ProjectedState::InProgress, Some(stage)) => stage.as_str().to_string(),
        _ => coarse.as_str().to_string(),
    }
}

mod arrangement;
pub use arrangement::{Column, board_columns, column_of, group_by_column};

mod decision;
pub use decision::{Decision, advance};
