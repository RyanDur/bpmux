use crate::sync::{SourceAdapter, is_permanent_push_failure};

use super::super::{
    DrainOneOutcome, MutationId, SyncOrchestrator, decrement_pending, record_dropped, record_error,
    record_push_success,
};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn drain_one_by_id(&self, id: MutationId) -> DrainOneOutcome {
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::NotFound;
        };
        let event = queued.event.clone();
        let lock = self.issue_lock(event.story_id().value()).await;
        let _guard = lock.lock().await;
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::Confirmed;
        };
        let event = queued.event.clone();
        match self
            .adapter
            .push_event(&event, &self.push_context_for(&event))
            .await
        {
            Ok(follow_up) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                record_push_success(&self.status);
                if let Some(refresh_event) = follow_up {
                    self.apply_story_event(&refresh_event);
                }
                DrainOneOutcome::Confirmed
            }
            Err(e) => {
                record_error(&self.status, &format!("push: {e}"));
                if is_permanent_push_failure(&e) {
                    self.remove_pending_by_id(id);
                    self.persist_outbox();
                    decrement_pending(&self.status);
                    record_dropped(&self.status, event.story_id().value(), format!("{e}"));
                    crate::diag::log_diag(&format!(
                        "orchestrator: drain_one_by_id dropped #{} after permanent push failure ({e})",
                        event.story_id().value()
                    ));
                    DrainOneOutcome::Confirmed
                } else {
                    DrainOneOutcome::TransientError(e)
                }
            }
        }
    }
}
