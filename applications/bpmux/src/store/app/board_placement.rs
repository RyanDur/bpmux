use std::collections::HashSet;
use std::sync::Arc;

use crate::board::{BoardAction, BoardPlacements, placed_story_ids};
use crate::product_management::story::slice::Stories;
use crate::product_management::story::{StoryId, Work};

pub(super) fn unplace_departed(stories: &Stories, board: &BoardPlacements) -> Vec<BoardAction> {
    let in_middle: HashSet<StoryId> = stories
        .values()
        .filter(|story| matches!(story.state, Work::InProgress))
        .map(|story| story.id)
        .collect();
    placed_story_ids(board)
        .into_iter()
        .filter(|story| !in_middle.contains(story))
        .map(|story| BoardAction::StoryUnplaced { story })
        .collect()
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
        let changes = unplace_departed(&reaction.current.stories, &reaction.current.board);
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

pub struct BoardPlacementPersister {
    session: Arc<crate::session::Session>,
}

impl BoardPlacementPersister {
    pub fn new(session: Arc<crate::session::Session>) -> Self {
        Self { session }
    }
}

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for BoardPlacementPersister
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.board, &reaction.current.board) {
            return Box::pin(async {});
        }
        let placements = reaction.current.board.clone();
        let dir = self.session.dir.clone();
        Box::pin(async move {
            if let Err(e) = crate::board::persistence::save(&dir, &placements) {
                crate::diag::log_diag(&format!("board: placement persist failed: {e}"));
            }
        })
    }
}
