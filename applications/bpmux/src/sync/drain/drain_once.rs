use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::sync::{SourceAdapter, SyncError, is_permanent_push_failure};

use super::super::{
    ConflictKind, ConflictResolveOrigin, SyncOrchestrator, cache_fill_meta, classify_conflict,
    decrement_pending, record_conflicts_pending, record_dropped, record_error, record_push_success,
};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn drain_once(&self) -> Result<usize, SyncError> {
        let mut drained = 0usize;
        let mut failures = 0usize;
        let mut conflicts_pending = 0usize;
        let mut conflict_offset = 0usize;
        loop {
            let queued_opt = self.with_pending(|p| p.get(conflict_offset).cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let lock = self.issue_lock(event.story_id().value()).await;
            let _guard = lock.lock().await;
            let queued_opt = self.with_pending(|p| p.get(conflict_offset).cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let current = self.read_story(event.story_id());
            let prior_classification = queued.last_classification;
            let kind = classify_conflict(queued.pre_state.as_ref(), &event, current.as_ref());

            if matches!(
                prior_classification,
                Some(
                    ConflictKind::DivergentState
                        | ConflictKind::DivergentLabel
                        | ConflictKind::External
                )
            ) && kind.is_none()
            {
                self.apply_story_event(&StoryEvent::new(
                    cache_fill_meta(),
                    event.story_id(),
                    StoryEventKind::ConflictResolved {
                        origin: ConflictResolveOrigin::DrainAutoReclassify,
                    },
                ));
                crate::diag::log_diag(&format!(
                    "orchestrator: conflict auto-resolved on drain re-classify for #{}",
                    event.story_id().value()
                ));
            }

            match kind {
                Some(ConflictKind::IdempotentSkip) => {
                    self.with_pending_mut(|p| {
                        if conflict_offset == 0 {
                            p.pop_front();
                        } else {
                            p.remove(conflict_offset);
                        }
                    });
                    self.persist_outbox();
                    decrement_pending(&self.status);
                    crate::diag::log_diag(&format!(
                        "orchestrator: idempotent-skip dropped #{} (already-applied at canonical store)",
                        event.story_id().value()
                    ));
                    continue;
                }
                Some(divergent) => {
                    if !matches!(
                        prior_classification,
                        Some(
                            ConflictKind::DivergentState
                                | ConflictKind::DivergentLabel
                                | ConflictKind::External
                        )
                    ) {
                        self.apply_story_event(&StoryEvent::new(
                            cache_fill_meta(),
                            event.story_id(),
                            StoryEventKind::ConflictDetected { kind: divergent },
                        ));
                        crate::diag::log_diag(&format!(
                            "orchestrator: conflict-detected {divergent:?} on #{}; left in outbox for slice-B click-to-resolve",
                            event.story_id().value()
                        ));
                    }
                    self.with_pending_mut(|p| {
                        if let Some(slot) = p.get_mut(conflict_offset) {
                            slot.last_classification = Some(divergent);
                        }
                    });
                    self.persist_outbox();
                    conflicts_pending += 1;
                    conflict_offset += 1;
                    continue;
                }
                None => {}
            }
            match self
                .adapter
                .push_event(&event, &self.push_context_for(&event))
                .await
            {
                Ok(follow_up) => {
                    self.with_pending_mut(|p| {
                        if conflict_offset == 0 {
                            p.pop_front();
                        } else {
                            p.remove(conflict_offset);
                        }
                    });
                    self.persist_outbox();
                    drained += 1;
                    record_push_success(&self.status);
                    if let Some(refresh_event) = follow_up {
                        self.apply_story_event(&refresh_event);
                    }
                }
                Err(e) => {
                    failures += 1;
                    record_error(&self.status, &format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.with_pending_mut(|p| {
                            if conflict_offset == 0 {
                                p.pop_front();
                            } else {
                                p.remove(conflict_offset);
                            }
                        });
                        self.persist_outbox();
                        decrement_pending(&self.status);
                        record_dropped(&self.status, event.story_id().value(), format!("{e}"));
                        crate::diag::log_diag(&format!(
                            "orchestrator: dropped event after permanent push failure ({e}); event #{}",
                            event.story_id().value()
                        ));
                        continue;
                    }
                    break;
                }
            }
        }
        record_conflicts_pending(&self.status, conflicts_pending);
        if failures > 0 {
            return Err(SyncError::Other(format!(
                "{failures} push failures; {drained} pushes succeeded"
            )));
        }
        Ok(drained)
    }
}
