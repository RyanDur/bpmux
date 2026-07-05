use std::collections::HashSet;

use crate::board::{BoardAction, BoardPlacements, placed_story_ids, placement_of};
use crate::product_management::story::BoardState;
use crate::product_management::story::slice::Stories;

pub(super) fn reconcile_placements(stories: &Stories, board: &BoardPlacements) -> Vec<BoardAction> {
    let desired: Vec<(
        crate::product_management::story::StoryId,
        crate::workflow::StageId,
    )> = stories
        .values()
        .filter_map(|story| match &story.state {
            BoardState::InProgress(stage) => Some((story.id, stage.clone())),
            _ => None,
        })
        .collect();
    let desired_ids: HashSet<_> = desired.iter().map(|(id, _)| *id).collect();

    let mut changes = Vec::new();
    for (story, stage) in &desired {
        if placement_of(board, *story).as_ref() != Some(stage) {
            changes.push(BoardAction::StoryPlaced {
                story: *story,
                stage: stage.clone(),
            });
        }
    }
    for story in placed_story_ids(board) {
        if !desired_ids.contains(&story) {
            changes.push(BoardAction::StoryUnplaced { story });
        }
    }
    changes
}

pub struct BoardPlacementReconciler;

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for BoardPlacementReconciler
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        let crate::store::AppAction::Story(_) = &reaction.action else {
            return Box::pin(async {});
        };
        let changes = reconcile_placements(&reaction.current.stories, &reaction.current.board);
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
