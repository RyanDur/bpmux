use std::sync::atomic::Ordering;

use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::{SourceAdapter, SyncError};

use super::{
    MutationId, QueuedEvent, SyncOrchestrator, coalesce_iteration_label_events, set_pending_events,
};

mod drain_by_id;
mod drain_once;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn mutate(&self, event: BoardEvent) -> Result<(), SyncError> {
        let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
        let story_id = event.story_id();
        let pre_state = self.store.backing_story(story_id);
        self.store.apply(&event);
        let queue_len = self.with_pending_mut(|p| {
            let absorbed = coalesce_iteration_label_events(&*self.codec, p, &event, id, &pre_state);
            if !absorbed {
                p.push_back(QueuedEvent::new_with_id(id, event, pre_state));
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

    pub(super) fn remove_pending_by_id(&self, id: MutationId) {
        self.with_pending_mut(|p| {
            if let Some(idx) = p.iter().position(|q| q.id == id) {
                p.remove(idx);
            }
        });
    }
}
