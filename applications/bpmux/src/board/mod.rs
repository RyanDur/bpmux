use std::collections::HashMap;

use crate::product_management::story::StoryId;
use crate::workflow::StageId;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod placement_tests;

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
