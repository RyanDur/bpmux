use crate::sync::SyncError;
use product_management::story::events::{StoryEvent, StoryEventKind};

use source_sync::PendingWrite;

use super::SyncOrchestrator;

mod drain_pass;
mod listener;

#[cfg(test)]
mod listener_tests;

pub use listener::GithubDrain;

impl SyncOrchestrator {
    pub fn mutate(&self, event: StoryEvent) -> Result<(), SyncError> {
        self.apply_story_event(&event);
        let write = PendingWrite::from_event(&*self.codec, event);
        self.dispatch_board(crate::store::AppAction::Sync(
            crate::store::SyncAction::WriteQueued { write },
        ));
        Ok(())
    }

    pub fn mutate_batch(&self, events: Vec<StoryEvent>) -> Result<(), SyncError> {
        if events.is_empty() {
            return Ok(());
        }
        self.apply_story_events(&events);
        let writes = events
            .into_iter()
            .map(|event| PendingWrite::from_event(&*self.codec, event))
            .collect();
        self.dispatch_board(crate::store::AppAction::Sync(
            crate::store::SyncAction::WritesQueued(writes),
        ));
        Ok(())
    }

    pub(super) fn push_context_for(&self, event: &StoryEvent) -> crate::sync::PushContext {
        crate::sync::PushContext {
            member_of_current_iteration: match &event.kind {
                StoryEventKind::StateTransitioned { .. } => {
                    product_management::scheduling::in_current_iteration(
                        event.id,
                        &self.read_iterations(),
                    )
                }
                _ => false,
            },
        }
    }
}
