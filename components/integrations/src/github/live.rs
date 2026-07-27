use std::sync::{Arc, RwLock};

use product_management::story::events::StoryEvent;
use source_sync::{
    BoxedFuture, Ingested, PushContext, SourceAdapter, StoryWithPlacement, SyncError,
};

use super::GitHubSourceAdapter;

#[derive(Clone)]
pub struct TrackerCell {
    current: Arc<RwLock<Option<Arc<GitHubSourceAdapter>>>>,
    observer: rate_limit::RateLimitObserver,
}

impl TrackerCell {
    pub fn seeded(adapter: Option<Arc<GitHubSourceAdapter>>) -> Self {
        Self {
            current: Arc::new(RwLock::new(adapter)),
            observer: rate_limit::RateLimitObserver::new(),
        }
    }

    pub fn observer(&self) -> rate_limit::RateLimitObserver {
        self.observer.clone()
    }

    pub fn connect(&self, adapter: Arc<GitHubSourceAdapter>) {
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = Some(adapter);
    }

    pub fn disconnect(&self) {
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn current(&self) -> Option<Arc<GitHubSourceAdapter>> {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn is_connected(&self) -> bool {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }
}

pub struct SwappableSource {
    cell: TrackerCell,
}

impl SwappableSource {
    pub fn new(cell: TrackerCell) -> Self {
        Self { cell }
    }

    fn resolve(&self) -> Result<Arc<GitHubSourceAdapter>, SyncError> {
        self.cell
            .current()
            .ok_or_else(|| SyncError::SourceUnreachable("github not connected".to_string()))
    }
}

impl SourceAdapter for SwappableSource {
    fn ingest_metadata(&self) -> BoxedFuture<'_, Result<Ingested, SyncError>> {
        Box::pin(async move { self.resolve()?.ingest_metadata().await })
    }

    fn ingest_metadata_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<Ingested, SyncError>> {
        Box::pin(async move { self.resolve()?.ingest_metadata_since(since).await })
    }

    fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Option<StoryWithPlacement>, SyncError>> {
        Box::pin(async move { self.resolve()?.fetch_story_metadata(number).await })
    }

    fn fetch_story_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>> {
        Box::pin(async move { self.resolve()?.fetch_story_body(number).await })
    }

    fn fetch_story_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<source_sync::ConditionalBody, SyncError>> {
        Box::pin(async move {
            self.resolve()?
                .fetch_story_body_conditional(number, prior_etag)
                .await
        })
    }

    fn fetch_story_labels(&self, number: u64) -> BoxedFuture<'_, Result<Vec<String>, SyncError>> {
        Box::pin(async move { self.resolve()?.fetch_story_labels(number).await })
    }

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<product_management::story::FreeComment>, SyncError>> {
        Box::pin(async move { self.resolve()?.fetch_free_comments(number).await })
    }

    fn fetch_verdicts(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<product_management::story::CastVerdict>, SyncError>> {
        Box::pin(async move { self.resolve()?.fetch_verdicts(number).await })
    }

    fn push_event<'a>(
        &'a self,
        event: &'a StoryEvent,
        context: &'a PushContext,
    ) -> BoxedFuture<'a, Result<Option<StoryEvent>, SyncError>> {
        Box::pin(async move { self.resolve()?.push_event(event, context).await })
    }

    fn create_story<'a>(
        &'a self,
        title: &'a str,
        body: &'a str,
        initial_labels: &'a [&'a str],
    ) -> BoxedFuture<'a, Result<u64, SyncError>> {
        Box::pin(async move {
            self.resolve()?
                .create_story(title, body, initial_labels)
                .await
        })
    }
}
