use std::collections::HashSet;

use crate::board::{BoardAction, BoardPlacements, placed_story_ids, placement_of};
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::slice::{Stories, StoryAction};
use crate::product_management::story::{BoardState, StoryId};

pub(super) fn unplace_departed(stories: &Stories, board: &BoardPlacements) -> Vec<BoardAction> {
    let in_middle: HashSet<StoryId> = stories
        .values()
        .filter(|story| matches!(story.state, BoardState::InProgress(_)))
        .map(|story| story.id)
        .collect();
    placed_story_ids(board)
        .into_iter()
        .filter(|story| !in_middle.contains(story))
        .map(|story| BoardAction::StoryUnplaced { story })
        .collect()
}

pub(super) fn placement_edge(board: &BoardPlacements, action: &StoryAction) -> Option<BoardAction> {
    let StoryAction::Event(StoryEvent::StateTransitioned { id, placement, .. }) = action else {
        return None;
    };
    match placement {
        Some(stage) if placement_of(board, *id).as_ref() != Some(stage) => {
            Some(BoardAction::StoryPlaced {
                story: *id,
                stage: stage.clone(),
            })
        }
        None if placement_of(board, *id).is_some() => {
            Some(BoardAction::StoryUnplaced { story: *id })
        }
        _ => None,
    }
}

pub struct BoardPlacementReconciler;

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for BoardPlacementReconciler
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        let crate::store::AppAction::Story(story_action) = &reaction.action else {
            return Box::pin(async {});
        };
        let board = &reaction.current.board;
        let mut changes = placement_edge(board, story_action)
            .into_iter()
            .collect::<Vec<_>>();
        changes.extend(unplace_departed(&reaction.current.stories, board));
        if changes.is_empty() {
            return Box::pin(async {});
        }
        let dispatch = std::sync::Arc::clone(&reaction.dispatch);
        Box::pin(async move {
            for action in changes {
                dispatch(crate::store::AppAction::Board(action));
            }
        })
    }
}
