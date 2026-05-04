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

/// #1165 slice 3: result of a conditional `If-None-Match` body fetch.
/// `NotModified` carries no payload — the canonical store is already
/// up-to-date, no event dispatched, no quota burned. `Modified` carries
/// the fresh body + the new ETag so the reducer can seed both.
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
    ) -> impl std::future::Future<Output = Result<Vec<Story>, SyncError>> + Send;

    /// #1165 slice 3: incremental list-fetch — pass the timestamp of
    /// the previous successful list-poll as `since`. GitHub's `?since=`
    /// filter returns only issues whose `updated_at` is at or after
    /// the supplied timestamp, so steady-state polls receive ~0 issues
    /// when no GH-side changes occurred. Default impl wraps
    /// `ingest_metadata` (full poll) for adapters that don't model
    /// delta queries; the github adapter overrides to add the query
    /// param.
    fn ingest_metadata_since(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl std::future::Future<Output = Result<Vec<Story>, SyncError>> + Send {
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

    /// #1165 slice 3: conditional fetch — pass the prior ETag for the
    /// story's body to skip transfer when the upstream copy hasn't
    /// changed (304 response). Default impl falls back to
    /// `fetch_story_body` for adapters that don't model HTTP cache
    /// validators (in-memory test substrate); the github adapter
    /// overrides to issue an `If-None-Match: <prior_etag>` request and
    /// surface `NotModified` on 304.
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
    ) -> impl std::future::Future<Output = Result<Vec<super::domain::FreeComment>, SyncError>> + Send;

    fn fetch_gate_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<super::domain::GateComment>, SyncError>> + Send;

    /// #1151: returns `Some(BoardEvent)` when GH's mutation response
    /// payload was captured; the orchestrator drain dispatches the
    /// returned event via `BoardStore::apply`. The follow-up is
    /// typically `StoryRefreshed { story }` for Issue-returning
    /// mutations or `LabelsRefreshed { labels }` for label mutations.
    /// Returns `None` for events with no useful response (e.g.,
    /// reducer-only events) or when the adapter chose not to capture
    /// (e.g., test in-memory adapter).
    fn push_event(
        &self,
        event: &BoardEvent,
    ) -> impl std::future::Future<Output = Result<Option<BoardEvent>, SyncError>> + Send;

    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<u64, SyncError>> + Send;

    /// Replace an existing story's body. The implementation pushes the
    /// new body to the source-of-truth (GitHub for production, the
    /// in-memory cache for tests).
    ///
    /// #1151: Returns the post-update Story so the orchestrator can
    /// dispatch `BoardEvent::StoryRefreshed` to the canonical store
    /// without a follow-up `refresh_story(N)` call. The github adapter
    /// parses octocrab's update-response Issue; the in-memory adapter
    /// returns the local Story with the updated body.
    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<super::domain::Story, SyncError>> + Send;
}
