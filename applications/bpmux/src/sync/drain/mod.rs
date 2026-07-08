use std::sync::atomic::Ordering;

use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::sync::{SourceAdapter, SyncError};

use super::{MutationId, QueuedWrite, SyncOrchestrator};

mod drain_pass;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn mutate(&self, event: StoryEvent) -> Result<(), SyncError> {
        let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
        self.apply_story_event(&event);
        let write = QueuedWrite::from_event(&*self.codec, id, event);
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
            .map(|event| {
                let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
                QueuedWrite::from_event(&*self.codec, id, event)
            })
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
                    crate::product_management::scheduling::in_current_iteration(
                        event.id,
                        &self.read_iterations(),
                    )
                }
                _ => false,
            },
        }
    }
}
