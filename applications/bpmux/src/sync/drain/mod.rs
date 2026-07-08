use std::sync::atomic::Ordering;

use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::sync::{SourceAdapter, SyncError};

use super::{
    MutationId, QueuedEvent, SyncOrchestrator, coalesce_iteration_label_events, set_pending_events,
};

mod drain_pass;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn mutate(&self, event: StoryEvent) -> Result<(), SyncError> {
        let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
        self.apply_story_event(&event);
        let queue_len = self.with_pending_mut(|p| {
            let absorbed = coalesce_iteration_label_events(&*self.codec, p, &event, id);
            if !absorbed {
                p.push_back(QueuedEvent::new_with_id(id, event));
            }
            p.len()
        });
        self.persist_outbox();
        set_pending_events(&self.status, queue_len);
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(crate::store::AppAction::Sync(
                crate::store::SyncAction::StartUpdateGithub { mutation_id: id },
            ));
        }
        Ok(())
    }

    pub fn mutate_batch(&self, events: Vec<StoryEvent>) -> Result<(), SyncError> {
        if events.is_empty() {
            return Ok(());
        }
        self.apply_story_events(&events);
        let mut ids = Vec::with_capacity(events.len());
        let queue_len = self.with_pending_mut(|p| {
            for event in events.iter() {
                let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
                ids.push(id);
                let absorbed = coalesce_iteration_label_events(&*self.codec, p, event, id);
                if !absorbed {
                    p.push_back(QueuedEvent::new_with_id(id, event.clone()));
                }
            }
            p.len()
        });
        self.persist_outbox();
        set_pending_events(&self.status, queue_len);
        if let Some(tx) = &self.action_tx {
            for id in ids {
                let _ = tx.send(crate::store::AppAction::Sync(
                    crate::store::SyncAction::StartUpdateGithub { mutation_id: id },
                ));
            }
        }
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
