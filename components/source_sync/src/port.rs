use product_management::epic::Epic;
use product_management::iteration::Iteration;
use product_management::outcome::Outcome;
use product_management::story::events::StoryEvent;
use product_management::story::{Story, StoryId};
use workflow::StageId;

#[derive(Debug, Clone, Default)]
pub struct Ingested {
    pub items: Vec<Story>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
    pub placements: Vec<(StoryId, StageId)>,
}

#[derive(Debug, Clone)]
pub enum SyncError {
    SourceUnreachable(String),
    RateLimited { retry_after_seconds: u64 },
    ValidationRejected(String),
    AlreadyClosed,
    Other(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::SourceUnreachable(s) => write!(f, "source unreachable: {s}"),
            SyncError::RateLimited {
                retry_after_seconds,
            } => write!(f, "rate-limited; retry after {retry_after_seconds}s"),
            SyncError::ValidationRejected(s) => write!(f, "validation rejected: {s}"),
            SyncError::AlreadyClosed => write!(f, "issue already closed"),
            SyncError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for SyncError {}

pub fn classify_close_error(msg: String) -> SyncError {
    if msg.contains("already closed") || msg.contains("is closed") {
        SyncError::AlreadyClosed
    } else {
        SyncError::Other(msg)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushContext {
    pub member_of_current_iteration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalBody {
    NotModified,
    Modified { body: String, etag: Option<String> },
}

pub fn is_permanent_push_failure(err: &SyncError) -> bool {
    matches!(
        err,
        SyncError::ValidationRejected(_) | SyncError::AlreadyClosed
    )
}

pub type BoxedFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

pub type StoryWithPlacement = (Story, Option<StageId>);

pub trait SourceAdapter: Send + Sync {
    fn ingest_metadata(&self) -> BoxedFuture<'_, Result<Ingested, SyncError>>;

    fn ingest_metadata_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> BoxedFuture<'_, Result<Ingested, SyncError>> {
        self.ingest_metadata()
    }

    fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Option<StoryWithPlacement>, SyncError>>;

    fn fetch_story_body(&self, number: u64) -> BoxedFuture<'_, Result<String, SyncError>>;

    fn fetch_story_body_conditional(
        &self,
        number: u64,
        _prior_etag: Option<String>,
    ) -> BoxedFuture<'_, Result<ConditionalBody, SyncError>> {
        Box::pin(async move {
            self.fetch_story_body(number)
                .await
                .map(|body| ConditionalBody::Modified { body, etag: None })
        })
    }

    fn fetch_story_labels(&self, number: u64) -> BoxedFuture<'_, Result<Vec<String>, SyncError>>;

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<product_management::story::FreeComment>, SyncError>>;

    fn fetch_verdicts(
        &self,
        number: u64,
    ) -> BoxedFuture<'_, Result<Vec<product_management::story::CastVerdict>, SyncError>>;

    fn push_event<'a>(
        &'a self,
        event: &'a StoryEvent,
        context: &'a PushContext,
    ) -> BoxedFuture<'a, Result<Option<StoryEvent>, SyncError>>;

    fn create_story<'a>(
        &'a self,
        title: &'a str,
        body: &'a str,
        initial_labels: &'a [&'a str],
    ) -> BoxedFuture<'a, Result<u64, SyncError>>;
}
