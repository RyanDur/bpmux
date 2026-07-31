use std::collections::HashSet;

use product_management::story::events::StoryEvent;
use source_sync::MutationId;

use crate::{SyncError, is_permanent_push_failure};

use super::super::SyncOrchestrator;

enum DrainStep {
    Pushed,
    Dropped,
    Halt,
}

impl SyncOrchestrator {
    pub async fn drain_pass(&self) -> Result<usize, SyncError> {
        let _flight = self.drain_guard.lock().await;
        let mut drained = 0usize;
        let mut failures = 0usize;
        let mut skip: HashSet<MutationId> = HashSet::new();
        while let Some(step) = self.push_head(&mut skip).await {
            match step {
                DrainStep::Pushed => drained += 1,
                DrainStep::Dropped => failures += 1,
                DrainStep::Halt => {
                    failures += 1;
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

    async fn push_head(&self, skip: &mut HashSet<MutationId>) -> Option<DrainStep> {
        let head = self.read_next_unpushed(skip)?;
        let lock = self.issue_lock(head.event.story_id().value()).await;
        let _guard = lock.lock().await;
        let queued = self.read_next_unpushed(skip)?;
        let event = queued.event.clone();
        match self
            .adapter
            .push_event(&event, &self.push_context_for(&event))
            .await
        {
            Ok(follow_up) => {
                self.absorb_push_success(queued.id, &event, follow_up.as_ref(), skip);
                Some(DrainStep::Pushed)
            }
            Err(error) => {
                self.record_error(&format!("push: {error}"));
                if !is_permanent_push_failure(&error) {
                    return Some(DrainStep::Halt);
                }
                self.drop_after_permanent_failure(&error, queued.id, &event, skip);
                Some(DrainStep::Dropped)
            }
        }
    }

    fn absorb_push_success(
        &self,
        queued_id: MutationId,
        event: &StoryEvent,
        follow_up: Option<&StoryEvent>,
        skip: &mut HashSet<MutationId>,
    ) {
        self.dispatch_write_pushed(queued_id);
        skip.insert(queued_id);
        self.record_push_success();
        let Some(refresh_event) = follow_up else {
            return;
        };
        let story_id = event.story_id();
        let queued_for_story = self.read_queued_for_story_excluding(story_id, skip);
        if queued_for_story == 0 {
            self.apply_story_event(refresh_event);
            return;
        }
        diag::log_diag(&format!(
            "orchestrator: refresh echo for #{} deferred to {queued_for_story} queued local write(s)",
            story_id.value()
        ));
    }

    fn drop_after_permanent_failure(
        &self,
        error: &SyncError,
        queued_id: MutationId,
        event: &StoryEvent,
        skip: &mut HashSet<MutationId>,
    ) {
        self.dispatch_write_dropped(queued_id);
        skip.insert(queued_id);
        self.record_dropped(event.story_id().value(), format!("{error}"));
        diag::log_diag(&format!(
            "orchestrator: dropped event after permanent push failure ({error}); event #{}",
            event.story_id().value()
        ));
    }
}
