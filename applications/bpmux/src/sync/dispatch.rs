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
                } else {
                    crate::diag::log_diag(
                        "orchestrator: board dispatch unbound; write dropped (bind_board_dispatch missing)",
                    )
                }
            }
        }
    }

    pub(super) fn dispatch_write_pushed(&self, id: crate::sync::MutationId) {
        self.dispatch_board(crate::store::AppAction::Sync(
            crate::store::SyncAction::WritePushed { id },
        ));
    }

    pub(super) fn dispatch_write_dropped(&self, id: crate::sync::MutationId) {
        self.dispatch_board(crate::store::AppAction::Sync(
            crate::store::SyncAction::WriteDropped { id },
        ));
    }

    pub(super) fn apply_story_event(&self, event: &StoryEvent) {
        self.dispatch_story(crate::product_management::story::slice::StoryAction::Event(
            event.clone(),
        ));
    }

    pub(super) fn apply_story_events(&self, events: &[StoryEvent]) {
        self.dispatch_story(
            crate::product_management::story::slice::StoryAction::Events(events.to_vec()),
        );
    }
}
