use crate::sync::{SourceAdapter, SyncError, is_permanent_push_failure};

use super::super::{
    SyncOrchestrator, decrement_pending, record_dropped, record_error, record_push_success,
};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn drain_once(&self) -> Result<usize, SyncError> {
        let mut drained = 0usize;
        let mut failures = 0usize;
        loop {
            let queued_opt = self.with_pending(|p| p.front().cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            let lock = self.issue_lock(event.story_id().value()).await;
            let _guard = lock.lock().await;
            let queued_opt = self.with_pending(|p| p.front().cloned());
            let Some(queued) = queued_opt else {
                break;
            };
            let event = queued.event.clone();
            match self
                .adapter
                .push_event(&event, &self.push_context_for(&event))
                .await
            {
                Ok(follow_up) => {
                    self.with_pending_mut(|p| {
                        p.pop_front();
                    });
                    self.persist_outbox();
                    drained += 1;
                    record_push_success(&self.status);
                    if let Some(refresh_event) = follow_up {
                        let story_id = event.story_id();
                        let queued_for_story = self.with_pending(|p| {
                            p.iter()
                                .filter(|q| q.event.story_id().value() == story_id.value())
                                .count()
                        });
                        if queued_for_story == 0 {
                            self.apply_story_event(&refresh_event);
                        } else {
                            crate::diag::log_diag(&format!(
                                "orchestrator: refresh echo for #{} deferred to {queued_for_story} queued local write(s)",
                                story_id.value()
                            ));
                        }
                    }
                }
                Err(e) => {
                    failures += 1;
                    record_error(&self.status, &format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.with_pending_mut(|p| {
                            p.pop_front();
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
        if failures > 0 {
            return Err(SyncError::Other(format!(
                "{failures} push failures; {drained} pushes succeeded"
            )));
        }
        Ok(drained)
    }
}
