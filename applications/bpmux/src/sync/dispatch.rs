use chrono::Utc;

use crate::product_management::story::events::StoryEvent;
use crate::store::SyncHealthAction;
use crate::sync::SourceAdapter;

use super::SyncOrchestrator;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn bind_dispatch(&self, dispatch: crate::store::Dispatch) {
        let _ = self.dispatch.set(dispatch);
    }

    fn emit(&self, action: crate::store::AppAction) {
        match self.dispatch.get() {
            Some(dispatch) => dispatch.dispatch(action),
            None => diag::log_diag(
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

    pub(super) fn record_ingest_success(&self) {
        self.emit(crate::store::AppAction::SyncHealth(
            SyncHealthAction::IngestSucceeded { at: Utc::now() },
        ));
    }

    pub(super) fn record_push_success(&self) {
        self.emit(crate::store::AppAction::SyncHealth(
            SyncHealthAction::PushSucceeded { at: Utc::now() },
        ));
    }

    pub(super) fn record_error(&self, msg: &str) {
        self.emit(crate::store::AppAction::SyncHealth(
            SyncHealthAction::ErrorRecorded {
                msg: msg.to_string(),
            },
        ));
    }

    pub(super) fn record_dropped(&self, story_id: u64, error: String) {
        self.emit(crate::store::AppAction::SyncHealth(
            SyncHealthAction::EventDropped {
                story_id,
                error,
                ts: Utc::now(),
            },
        ));
    }

    pub fn acknowledge_dropped(&self) -> usize {
        let cleared = self.read_dropped_count();
        self.emit(crate::store::AppAction::SyncHealth(
            SyncHealthAction::DroppedAcknowledged,
        ));
        cleared
    }
}
