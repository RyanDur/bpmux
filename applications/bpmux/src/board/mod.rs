use std::collections::HashMap;

use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use crate::workflow::{StageId, WorkflowConfig};

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod placement_tests;
#[cfg(test)]
mod verdict_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardAction {
    StoryPlaced { story: StoryId, stage: StageId },
    StoryUnplaced { story: StoryId },
}

#[derive(Default, Clone)]
pub struct BoardPlacements {
    placements: HashMap<StoryId, StageId>,
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

pub fn fine_stage(story: &Story, board: &BoardPlacements) -> Option<Stage> {
    match &story.state {
        BoardState::InProgress => Some(
            placement_of(board, story.id)
                .and_then(|stage_id| Stage::from_id(&stage_id))
                .unwrap_or(Stage::Dev),
        ),
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
            BoardState::InProgress => Some(match desk_check.get(&story.id) {
                Some(stage) => BoardAction::StoryPlaced {
                    story: story.id,
                    stage: (*stage).clone(),
                },
                None => BoardAction::StoryUnplaced { story: story.id },
            }),
            _ => None,
        })
        .collect()
}

pub fn state_label(coarse: &BoardState, fine: Option<Stage>) -> &'static str {
    match fine {
        Some(Stage::DeskCheck) => "desk-check",
        Some(Stage::Dev) => "in-progress",
        None => coarse.as_str(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Succeed { step: StageId },
    Fail { step: StageId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub coarse: BoardState,
    pub placement: Option<StageId>,
}

fn state_position(id: &str, workflow: &WorkflowConfig) -> Option<usize> {
    workflow.state.iter().position(|s| s.id == id)
}

fn next_state_id_in_order(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    let pos = state_position(from_id, workflow)?;
    workflow.state.get(pos + 1).map(|s| s.id.clone())
}

fn backward_target_id(from_id: &str, workflow: &WorkflowConfig) -> Option<String> {
    let from_pos = state_position(from_id, workflow)?;
    workflow
        .transition
        .iter()
        .find(|t| {
            t.from == from_id
                && state_position(&t.to, workflow).is_some_and(|to_pos| to_pos < from_pos)
        })
        .map(|t| t.to.clone())
}

pub fn advance(verdict: &Verdict, workflow: &WorkflowConfig) -> Option<Decision> {
    let target_id = match verdict {
        Verdict::Succeed { step } => next_state_id_in_order(step.as_str(), workflow)?,
        Verdict::Fail { step } => backward_target_id(step.as_str(), workflow)?,
    };
    let coarse = BoardState::parse(&target_id)?;
    let middle = crate::workflow::board_middle_stage_ids(workflow);
    Some(Decision {
        coarse,
        placement: placement_from_target(&target_id, &middle),
    })
}
