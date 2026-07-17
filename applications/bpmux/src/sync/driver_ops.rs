use crate::sync::{ConditionalBody, SourceAdapter, SyncError};
use product_management::story::events::StoryEvent;
use product_management::story::{CastVerdict, Story, StoryId};

use source_sync::SyncStatus;

use super::{BoxedFuture, RefreshDriver, SyncOrchestrator};

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

    fn outcomes(&self) -> Vec<product_management::outcome::Outcome> {
        self.read_outcomes()
    }

    fn iterations(&self) -> Vec<product_management::iteration::Iteration> {
        self.read_iterations()
    }

    fn snapshot_status(&self) -> SyncStatus {
        self.snapshot_status()
    }

    fn acknowledge_dropped(&self) -> usize {
        self.acknowledge_dropped()
    }

    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>> {
        Box::pin(self.drain_pass())
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

    fn all_stories(&self) -> Vec<Story> {
        self.read_all_stories()
    }

    fn store_is_empty(&self) -> bool {
        self.read_store_is_empty()
    }

    fn bind_dispatch(&self, dispatch: crate::store::Dispatch) {
        self.bind_dispatch(dispatch);
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
