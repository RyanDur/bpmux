use crate::product_management::story::events::StoryEvent;
use crate::sync::{SourceAdapter, is_permanent_push_failure};

use super::super::{
    ConflictKind, DrainOneOutcome, MutationId, SyncOrchestrator, cache_fill_meta,
    classify_conflict, decrement_pending, record_dropped, record_error, record_push_success,
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
        let current = self.read_story(event.story_id());
        let kind = classify_conflict(queued.pre_state.as_ref(), &event, current.as_ref());
        match kind {
            Some(ConflictKind::IdempotentSkip) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                crate::diag::log_diag(&format!(
                    "orchestrator: idempotent-skip dropped #{} via drain_one_by_id (already-applied)",
                    event.story_id().value()
                ));
                DrainOneOutcome::Confirmed
            }
            Some(divergent) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                self.apply_story_event(&StoryEvent::ConflictDetected {
                    meta: cache_fill_meta(),
                    id: event.story_id(),
                    kind: divergent,
                });
                crate::diag::log_diag(&format!(
                    "orchestrator: drain_one_by_id rolled back #{} ({divergent:?}); applying server_story",
                    event.story_id().value()
                ));
                DrainOneOutcome::RolledBack {
                    server_story: current.map(Box::new),
                }
            }
            None => match self
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
            },
        }
    }
}
