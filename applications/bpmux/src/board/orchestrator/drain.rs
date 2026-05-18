use std::sync::atomic::Ordering;

use crate::board::events::BoardEvent;
use crate::board::sync::{SourceAdapter, SyncError, is_permanent_push_failure};

use super::{
    ConflictKind, ConflictResolveOrigin, DrainOneOutcome, MutationId, QueuedEvent,
    SyncOrchestrator, cache_fill_meta, classify_conflict, coalesce_iteration_label_events,
    decrement_pending, record_conflicts_pending, record_dropped, record_error, record_push_success,
    set_pending_events,
};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn mutate(&self, event: BoardEvent) -> Result<(), SyncError> {
        let id = MutationId(self.next_mutation_id.fetch_add(1, Ordering::SeqCst));
        let pre_state = self.store.story(event.story_number());
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
            let _ = tx.send(crate::orchestrator::OrchestratorAction::StartUpdateGithub {
                mutation_id: id,
            });
        }
        Ok(())
    }

    pub async fn drain_one_by_id(&self, id: MutationId) -> DrainOneOutcome {
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::NotFound;
        };
        let event = queued.event.clone();
        let lock = self.issue_lock(event.story_number()).await;
        let _guard = lock.lock().await;
        let queued = self.with_pending(|p| p.iter().find(|q| q.id == id).cloned());
        let Some(queued) = queued else {
            return DrainOneOutcome::Confirmed;
        };
        let event = queued.event.clone();
        let current = self.store.story(event.story_number());
        let kind = classify_conflict(queued.pre_state.as_ref(), &event, current.as_ref());
        match kind {
            Some(ConflictKind::IdempotentSkip) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                crate::session::diag::log_diag(&format!(
                    "orchestrator: idempotent-skip dropped #{} via drain_one_by_id (already-applied)",
                    event.story_number()
                ));
                DrainOneOutcome::Confirmed
            }
            Some(divergent) => {
                self.remove_pending_by_id(id);
                self.persist_outbox();
                decrement_pending(&self.status);
                self.store.apply(&BoardEvent::ConflictDetected {
                    meta: cache_fill_meta(),
                    number: event.story_number(),
                    kind: divergent,
                });
                crate::session::diag::log_diag(&format!(
                    "orchestrator: drain_one_by_id rolled back #{} ({divergent:?}); applying server_story",
                    event.story_number()
                ));
                DrainOneOutcome::RolledBack {
                    server_story: current.map(Box::new),
                }
            }
            None => match self.adapter.push_event(&event).await {
                Ok(follow_up) => {
                    self.remove_pending_by_id(id);
                    self.persist_outbox();
                    decrement_pending(&self.status);
                    record_push_success(&self.status);
                    if let Some(refresh_event) = follow_up {
                        self.store.apply(&refresh_event);
                    }
                    DrainOneOutcome::Confirmed
                }
                Err(e) => {
                    record_error(&self.status, &format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.remove_pending_by_id(id);
                        self.persist_outbox();
                        decrement_pending(&self.status);
                        record_dropped(&self.status, event.story_number(), format!("{e}"));
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: drain_one_by_id dropped #{} after permanent push failure ({e})",
                            event.story_number()
                        ));
                        DrainOneOutcome::Confirmed
                    } else {
                        DrainOneOutcome::TransientError(e)
                    }
                }
            },
        }
    }

    pub(super) fn remove_pending_by_id(&self, id: MutationId) {
        self.with_pending_mut(|p| {
            if let Some(idx) = p.iter().position(|q| q.id == id) {
                p.remove(idx);
            }
        });
    }

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
            let lock = self.issue_lock(event.story_number()).await;
            let _guard = lock.lock().await;
            let queued_opt = self.with_pending(|p| p.get(conflict_offset).cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let current = self.store.story(event.story_number());
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
                self.store.apply(&BoardEvent::ConflictResolved {
                    meta: cache_fill_meta(),
                    number: event.story_number(),
                    origin: ConflictResolveOrigin::DrainAutoReclassify,
                });
                crate::session::diag::log_diag(&format!(
                    "orchestrator: conflict auto-resolved on drain re-classify for #{}",
                    event.story_number()
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
                    crate::session::diag::log_diag(&format!(
                        "orchestrator: idempotent-skip dropped #{} (already-applied at canonical store)",
                        event.story_number()
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
                        self.store.apply(&BoardEvent::ConflictDetected {
                            meta: cache_fill_meta(),
                            number: event.story_number(),
                            kind: divergent,
                        });
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: conflict-detected {divergent:?} on #{}; left in outbox for slice-B click-to-resolve",
                            event.story_number()
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
            match self.adapter.push_event(&event).await {
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
                        self.store.apply(&refresh_event);
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
                        record_dropped(&self.status, event.story_number(), format!("{e}"));
                        crate::session::diag::log_diag(&format!(
                            "orchestrator: dropped event after permanent push failure ({e}); event #{}",
                            event.story_number()
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
