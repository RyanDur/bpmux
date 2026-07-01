use super::events::BoardEvent;
use super::item::ProductItem;
use crate::product_management::epic::Epic;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::Story;

#[derive(Debug, Clone, Default)]
pub struct Ingested {
    pub items: Vec<ProductItem>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
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

pub trait SourceAdapter: Send + Sync {
    fn ingest_metadata(
        &self,
    ) -> impl std::future::Future<Output = Result<Ingested, SyncError>> + Send;

    fn ingest_metadata_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl std::future::Future<Output = Result<Ingested, SyncError>> + Send {
        self.ingest_metadata()
    }

    fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Option<Story>, SyncError>> + Send;

    fn fetch_story_body(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<String, SyncError>> + Send;

    fn fetch_story_body_conditional(
        &self,
        number: u64,
        _prior_etag: Option<String>,
    ) -> impl std::future::Future<Output = Result<ConditionalBody, SyncError>> + Send {
        async move {
            self.fetch_story_body(number)
                .await
                .map(|body| ConditionalBody::Modified { body, etag: None })
        }
    }

    fn fetch_story_labels(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<String>, SyncError>> + Send;

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<
        Output = Result<Vec<crate::product_management::story::FreeComment>, SyncError>,
    > + Send;

    fn fetch_gate_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<
        Output = Result<Vec<crate::product_management::story::GateComment>, SyncError>,
    > + Send;

    fn push_event(
        &self,
        event: &BoardEvent,
    ) -> impl std::future::Future<Output = Result<Option<BoardEvent>, SyncError>> + Send;

    fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> impl std::future::Future<Output = Result<u64, SyncError>> + Send;

    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<
        Output = Result<crate::product_management::story::Story, SyncError>,
    > + Send;

    fn update_title(
        &self,
        number: u64,
        title: &str,
    ) -> impl std::future::Future<
        Output = Result<crate::product_management::story::Story, SyncError>,
    > + Send;
}
