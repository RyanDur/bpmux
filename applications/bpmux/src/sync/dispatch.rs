use crate::product_management::story::Story;
use crate::product_management::story::events::StoryEvent;
use crate::sync::SourceAdapter;

use super::SyncOrchestrator;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn bind_story_dispatch(
        &self,
        dispatch: impl Fn(crate::product_management::story::slice::StoryAction) + Send + Sync + 'static,
    ) {
        let _ = self.story_dispatch.set(Box::new(dispatch));
    }

    pub(super) fn dispatch_story(
        &self,
        action: crate::product_management::story::slice::StoryAction,
    ) {
        match self.story_dispatch.get() {
            Some(dispatch) => dispatch(action),
            None => crate::diag::log_diag(
                "orchestrator: story dispatch unbound; write dropped (bind_story_dispatch missing)",
            ),
        }
    }

    pub fn bind_board_dispatch(
        &self,
        dispatch: impl Fn(crate::store::AppAction) + Send + Sync + 'static,
    ) {
        let _ = self.board_dispatch.set(Box::new(dispatch));
    }

    pub(super) fn dispatch_board(&self, action: crate::store::AppAction) {
        match self.board_dispatch.get() {
            Some(dispatch) => dispatch(action),
            None => {
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(action);
                }
            }
        }
    }

    pub(super) fn apply_story_event(&self, event: &StoryEvent) {
        self.dispatch_story(crate::product_management::story::slice::StoryAction::Event(
            event.clone(),
        ));
    }

    pub fn apply_server_rollback(&self, story: Story) {
        self.dispatch_story(
            crate::product_management::story::slice::StoryAction::Seeded(vec![story]),
        );
    }
}
