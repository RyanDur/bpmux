use std::sync::Arc;

use crate::sync::{ConditionalBody, SyncError};
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::Outcome;
use product_management::story::events::StoryEvent;
use product_management::story::{CastVerdict, Story};

use source_sync::SyncStatus;

use super::BoxedFuture;

pub type OrchestratorHandle = Option<Arc<dyn RefreshDriver>>;

pub trait RefreshDriver: Send + Sync {
    fn bind_select(&self, _select: crate::store::Select) {}
    fn bind_dispatch(&self, _dispatch: crate::store::Dispatch) {}
    fn refresh_story(&self, id: u64) -> BoxedFuture<'_, Result<Option<Story>, SyncError>>;
    fn refresh_all(&self) -> BoxedFuture<'_, Result<(), SyncError>>;
    fn refresh_all_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<(), SyncError>> {
        self.refresh_all()
    }
    fn mutate_sync(&self, event: StoryEvent) -> Result<(), SyncError>;

    fn mutate_batch_sync(&self, events: Vec<StoryEvent>) -> Result<(), SyncError>;
    fn snapshot_status(&self) -> SyncStatus;
    fn acknowledge_dropped(&self) -> usize;
    fn drain_pending(&self) -> BoxedFuture<'_, Result<usize, SyncError>>;
    fn fetch_body(&self, id: u64) -> BoxedFuture<'_, Result<String, SyncError>>;
    fn fetch_body_conditional(
        &self,
        id: u64,
        _prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(async move {
            self.fetch_body(id)
                .await
                .map(|body| ConditionalBody::Modified { body, etag: None })
        })
    }
    fn fetch_verdicts(&self, id: u64) -> BoxedFuture<'_, Result<Vec<CastVerdict>, SyncError>>;
    fn create_story(
        &self,
        title: String,
        body: String,
        initial_labels: Vec<String>,
    ) -> BoxedFuture<'_, Result<u64, SyncError>>;
    fn story(&self, id: u64) -> Option<Story>;
    fn outcomes(&self) -> Vec<Outcome> {
        Vec::new()
    }
    fn iterations(&self) -> Vec<Iteration> {
        Vec::new()
    }
    fn iteration(&self, iteration_id: IterationId) -> Option<Iteration> {
        self.iterations()
            .into_iter()
            .find(|iteration| iteration.id == iteration_id)
    }
    fn remove_story(&self, id: u64) -> Option<Story>;
    fn clear_stories(&self);
    fn update_body(&self, id: u64, body: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn update_title(&self, id: u64, title: String) -> BoxedFuture<'_, Result<(), SyncError>>;

    fn test_race_window_rendezvous(&self) -> BoxedFuture<'_, ()> {
        Box::pin(async {})
    }

    fn test_race_window_complete(&self) {}
}
