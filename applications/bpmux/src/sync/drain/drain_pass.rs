use std::collections::HashSet;

use crate::sync::{MutationId, SourceAdapter, SyncError, is_permanent_push_failure};

use super::super::SyncOrchestrator;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn drain_pass(&self) -> Result<usize, SyncError> {
        let _flight = self.drain_guard.lock().await;
        let mut drained = 0usize;
        let mut failures = 0usize;
        let mut skip: HashSet<MutationId> = HashSet::new();
        while let Some(head) = self.read_next_unpushed(&skip) {
            let lock = self.issue_lock(head.event.story_id().value()).await;
            let _guard = lock.lock().await;
            let Some(queued) = self.read_next_unpushed(&skip) else {
                break;
            };
            let event = queued.event.clone();
            match self
                .adapter
                .push_event(&event, &self.push_context_for(&event))
                .await
            {
                Ok(follow_up) => {
                    self.dispatch_write_pushed(queued.id);
                    skip.insert(queued.id);
                    drained += 1;
                    self.record_push_success();
                    if let Some(refresh_event) = follow_up {
                        let story_id = event.story_id();
                        let queued_for_story =
                            self.read_queued_for_story_excluding(story_id, &skip);
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
                    self.record_error(&format!("push: {e}"));
                    if is_permanent_push_failure(&e) {
                        self.dispatch_write_dropped(queued.id);
                        skip.insert(queued.id);
                        self.record_dropped(event.story_id().value(), format!("{e}"));
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
