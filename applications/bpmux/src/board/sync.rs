use super::domain::Story;
use super::events::BoardEvent;

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

pub trait SourceAdapter: Send + Sync {
    fn ingest_metadata(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<Story>, SyncError>> + Send;

    fn fetch_story_metadata(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Option<Story>, SyncError>> + Send;

    fn fetch_story_body(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<String, SyncError>> + Send;

    fn fetch_story_labels(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<String>, SyncError>> + Send;

    fn fetch_free_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<super::domain::FreeComment>, SyncError>> + Send;

    fn fetch_gate_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<super::domain::GateComment>, SyncError>> + Send;

    fn push_event(
        &self,
        event: &BoardEvent,
    ) -> impl std::future::Future<Output = Result<(), SyncError>> + Send;

    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<u64, SyncError>> + Send;

    /// Replace an existing story's body. The implementation pushes the
    /// new body to the source-of-truth (GitHub for production, the
    /// in-memory cache for tests). Callers re-fetch the story after
    /// to observe the new body in their local BoardStore (#530 closes
    /// the gh-bypass class for body-edits).
    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<(), SyncError>> + Send;
}
