use chrono::Utc;

use product_management::story::events::StoryEvent;
use store::SyncHealthAction;

use super::SyncOrchestrator;

impl SyncOrchestrator {
    fn emit(&self, action: store::AppAction) {
        self.dispatch.dispatch(action);
    }

    pub(super) fn dispatch_story(&self, action: product_management::story::slice::StoryAction) {
        self.emit(store::AppAction::Story(action));
    }

    pub(super) fn dispatch_board(&self, action: store::AppAction) {
        self.emit(action);
    }

    pub(super) fn dispatch_write_pushed(&self, id: source_sync::MutationId) {
        self.dispatch_board(store::AppAction::Sync(store::SyncAction::WritePushed {
            id,
        }));
    }

    pub(super) fn dispatch_write_dropped(&self, id: source_sync::MutationId) {
        self.dispatch_board(store::AppAction::Sync(store::SyncAction::WriteDropped {
            id,
        }));
    }

    pub(super) fn apply_story_event(&self, event: &StoryEvent) {
        self.dispatch_story(product_management::story::slice::StoryAction::Event(
            event.clone(),
        ));
    }

    pub(super) fn apply_story_events(&self, events: &[StoryEvent]) {
        self.dispatch_story(product_management::story::slice::StoryAction::Events(
            events.to_vec(),
        ));
    }

    pub(super) fn record_ingest_success(&self) {
        self.emit(store::AppAction::SyncHealth(
            SyncHealthAction::IngestSucceeded { at: Utc::now() },
        ));
    }

    pub(super) fn record_push_success(&self) {
        self.emit(store::AppAction::SyncHealth(
            SyncHealthAction::PushSucceeded { at: Utc::now() },
        ));
    }

    pub(super) fn record_error(&self, msg: &str) {
        self.emit(store::AppAction::SyncHealth(
            SyncHealthAction::ErrorRecorded {
                msg: msg.to_string(),
            },
        ));
    }

    pub(super) fn record_dropped(&self, story_id: u64, error: String) {
        self.emit(store::AppAction::SyncHealth(
            SyncHealthAction::EventDropped {
                story_id,
                error,
                ts: Utc::now(),
            },
        ));
    }

    pub fn acknowledge_dropped(&self) -> usize {
        let cleared = self.read_dropped_count();
        self.emit(store::AppAction::SyncHealth(
            SyncHealthAction::DroppedAcknowledged,
        ));
        cleared
    }
}
