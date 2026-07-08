use std::sync::Arc;

use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{CastVerdict, FreeComment, Story, StoryId};
use crate::sync::{ConditionalBody, SourceAdapter, SyncError};

use super::{
    BoxedFuture, DrainOneOutcome, MutationId, RefreshDriver, SyncOrchestrator, SyncStatus,
    acknowledge_dropped,
};

impl<A: SourceAdapter + 'static> RefreshDriver for SyncOrchestrator<A> {
    fn refresh_story(&self, number: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>> {
        Box::pin(self.refresh_story(number))
    }

    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(self.refresh_all())
    }

    fn refresh_all_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(self.ingest_once_since(since))
    }

    fn mutate_sync(&self, event: StoryEvent) -> Result<(), SyncError> {
        self.mutate(event)
    }

    fn mutate_batch_sync(&self, events: Vec<StoryEvent>) -> Result<(), SyncError> {
        self.mutate_batch(events)
    }

    fn bind_select(&self, select: crate::store::Select) {
        self.set_select(select);
    }

    fn outcomes(&self) -> Vec<crate::product_management::outcome::Outcome> {
        self.read_outcomes()
    }

    fn iterations(&self) -> Vec<crate::product_management::iteration::Iteration> {
        self.read_iterations()
    }

    fn epics(&self) -> Vec<crate::product_management::epic::Epic> {
        self.read_epics()
    }

    fn drain_one_by_id(&self, id: MutationId) -> BoxedFuture<'_, DrainOneOutcome> {
        Box::pin(self.drain_one_by_id(id))
    }

    fn snapshot_pending_for_mutation(&self, id: MutationId) -> Option<u64> {
        self.pending_issue_for_mutation(id)
    }

    fn issue_lock(&self, issue: u64) -> BoxedFuture<'_, Arc<tokio::sync::Mutex<()>>> {
        Box::pin(self.issue_lock(issue))
    }

    fn snapshot_status(&self) -> SyncStatus {
        self.snapshot_status()
    }

    fn status_handle(&self) -> Arc<std::sync::Mutex<SyncStatus>> {
        self.status_handle()
    }

    fn acknowledge_dropped(&self) -> usize {
        acknowledge_dropped(&self.status)
    }

    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.drain_once())
    }

    fn reconcile_dual_state_labels(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.reconcile_dual_state_labels())
    }

    fn fetch_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>> {
        Box::pin(self.fetch_body(number))
    }

    fn fetch_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(self.fetch_body_conditional(number, prior_etag))
    }

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<FreeComment>, SyncError>> {
        Box::pin(self.fetch_free_comments(number))
    }

    fn fetch_verdicts(&self, number: u64) -> BoxedFuture<'_, Result<Vec<CastVerdict>, SyncError>> {
        Box::pin(self.fetch_verdicts(number))
    }

    fn create_story(
        &self,
        title: String,
        body: String,
        initial_labels: Vec<String>,
    ) -> BoxedFuture<'_, Result<u64, SyncError>> {
        Box::pin(async move {
            let label_refs: Vec<&str> = initial_labels.iter().map(|s| s.as_str()).collect();
            self.create_story(&title, &body, &label_refs).await
        })
    }

    fn story(&self, number: u64) -> Option<Story> {
        self.read_story(StoryId::new(number))
    }

    fn story_fine_stage(&self, story: &Story) -> Option<crate::product_management::story::Stage> {
        self.read_story_fine_stage(story)
    }

    fn story_held_stage(&self, story: &Story) -> Option<crate::workflow::StageId> {
        self.read_story_held_stage(story)
    }

    fn all_stories(&self) -> Vec<Story> {
        self.read_all_stories()
    }

    fn backing_story(&self, id: u64) -> Option<Story> {
        self.read_story(StoryId::new(id))
    }

    fn store_is_empty(&self) -> bool {
        self.read_store_is_empty()
    }

    fn bind_story_dispatch(
        &self,
        dispatch: Box<dyn Fn(crate::product_management::story::slice::StoryAction) + Send + Sync>,
    ) {
        self.bind_story_dispatch(dispatch);
    }

    fn bind_board_dispatch(&self, dispatch: Box<dyn Fn(crate::store::AppAction) + Send + Sync>) {
        self.bind_board_dispatch(dispatch);
    }

    fn remove_story(&self, id: u64) -> Option<Story> {
        self.remove_story(StoryId::new(id))
    }

    fn clear_stories(&self) {
        self.clear_stories();
    }

    fn update_body(&self, number: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(std::future::ready(self.update_body(number, body)))
    }

    fn update_title(&self, number: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>> {
        Box::pin(std::future::ready(self.update_title(number, title)))
    }

    fn ci_state(&self) -> Option<crate::workflow::CiSnapshot> {
        let store = self.ci_state_store.as_ref()?;
        let report = store.read()?;
        Some(crate::workflow::CiSnapshot {
            conclusion: report.conclusion.to_string(),
            head_sha: report.head_sha.unwrap_or_default(),
        })
    }

    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        let barrier = self.race_window_barrier.clone();
        Box::pin(async move {
            if let Some(b) = barrier {
                b.arrive().await;
            }
        })
    }

    fn test_race_window_complete(&self) {
        if let Some(b) = self.race_window_barrier.as_ref() {
            b.complete();
        }
    }
}
