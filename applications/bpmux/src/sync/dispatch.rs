use crate::product_management::story::events::StoryEvent;
use crate::sync::SourceAdapter;

use super::SyncOrchestrator;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn bind_dispatch(&self, dispatch: crate::store::Dispatch) {
        let _ = self.dispatch.set(dispatch);
    }

    fn emit(&self, action: crate::store::AppAction) {
        match self.dispatch.get() {
            Some(dispatch) => dispatch.dispatch(action),
            None => crate::diag::log_diag(
                "orchestrator: dispatch unbound; write dropped (bind_dispatch missing)",
            ),
        }
    }

    pub(super) fn dispatch_story(
        &self,
        action: crate::product_management::story::slice::StoryAction,
    ) {
        self.emit(crate::store::AppAction::Story(action));
    }

    pub(super) fn dispatch_board(&self, action: crate::store::AppAction) {
        self.emit(action);
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
