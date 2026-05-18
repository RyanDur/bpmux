//! The `StoryTracker` trait surface + cross-tracker types.
//!
//! `StoryTracker` is the bridge backend abstraction: every handler
//! that mutates a GH issue speaks to it instead of reaching directly
//! to `octocrab`. `WorkflowRunEntry` + `ConditionalIssue` +
//! `ConditionalRunEntry` are the shared fetch-result shapes the trait
//! returns. Extracted to its own module per #1411 INVESTIGATE so the
//! trait substrate is cleanly separable from the sole production
//! impl (`OctocrabStoryTracker` in the sister `octocrab` module).

pub(crate) use crate::github::ensure_crypto_provider_installed;

/// Shared fail-soft handler for `get_story_labels`: log the error
/// through the diag sink so it's observable via `BPMUX_LOG`, then
/// return an empty Vec. Caller treats empty as "ambiguous category"
/// and defaults the addressee up to Ryan (classifier fail-safe).
///
/// Factored out so both tracker impls emit the same diagnostic text
/// and unit tests can assert the observable behavior without needing
/// live gh/octocrab.
pub(crate) fn log_labels_fetch_failure<E: std::fmt::Display>(
    backend: &str,
    number: u64,
    err: E,
) -> Vec<String> {
    crate::session::diag::log_diag(&format!(
        "get_story_labels ({backend}) #{number} failed: {err}"
    ));
    Vec::new()
}

/// Lightweight representation of a single workflow run on GitHub
/// Actions — what `get_ci_state` returns and what `terminal::ci`
/// classifies. Single source of truth so the TUI poller + the MCP
/// tool share the same vocabulary (#320).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct WorkflowRunEntry {
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "head_sha", alias = "headSha", default)]
    pub head_sha: Option<String>,
    #[serde(rename = "display_title", alias = "displayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "id", alias = "databaseId", default)]
    pub database_id: Option<u64>,
    /// #1287: workflow name (e.g., "CI", "self-hosted runner
    /// smoke-test") used for per-workflow breakdown in aggregate
    /// state — GitHub API field on `/repos/.../actions/runs`.
    #[serde(default)]
    pub name: Option<String>,
    /// #397: annotation message from the check-run surfaced on
    /// failed runs so classification can distinguish quota-
    /// exhausted / billing-blocked (pass-with-citation) from genuine
    /// test/build failures (hard-block). Populated by
    /// `list_latest_run_on_main` for failed runs; `None` for
    /// successful runs (annotation fetch skipped).
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

pub trait StoryTracker: Clone + Send + Sync + 'static {
    /// #1151: returns the post-mutation Issue JSON so the adapter can
    /// parse it into a `Story` and dispatch `BoardEvent::StoryRefreshed`
    /// without a follow-up `refresh_story(N)` call. octocrab's
    /// `issues().update(N).state(Closed).send()` returns the updated
    /// Issue; we capture it here instead of discarding via `.map(|_|
    /// String)`.
    fn close_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-mutation Issue JSON (parallel to
    /// `close_story`).
    fn reopen_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-mutation labels list (octocrab's
    /// `add_labels` returns `Vec<Label>`). Capturing the labels list
    /// lets callers re-derive the local Story state via
    /// `state_from_labels` without a follow-up `get_story` round-trip.
    fn add_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    /// #1151: returns the post-mutation labels list. Empty Vec on the
    /// 404-idempotent path (label already absent).
    fn remove_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    /// #1151: returns the freshly-created Issue JSON. Includes the
    /// auto-assigned issue number; callers that previously parsed
    /// the format-string `"#{number} {title}"` should read
    /// `value["number"]` directly.
    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-update Issue JSON.
    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1279: sister to update_body at the title-axis. Returns the
    /// post-update Issue JSON for orchestrator-store apply.
    fn update_title(
        &self,
        number: u64,
        title: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #403: full board as typed JSON Values — used by `list_ready`
    /// and other filtered enumerations. Counterpart to the inherent
    /// `OctocrabStoryTracker::list_stories` (returns pre-serialized
    /// string); keeping both so callers that want the raw string
    /// (legacy contract tests) don't pay a re-serialize round-trip.
    fn fetch_board_stories(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send;
    /// #403: full detail for one story. Returns `Ok(None)` when the
    /// issue doesn't exist so handlers can map to a typed "not
    /// found" response instead of erroring on an expected absence.
    fn get_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
    fn comment_on_story(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn list_story_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send;
    /// Fetch labels for story-category routing (pass_desk_check
    /// signal picks addressee based on honest-plumbing vs vertical).
    /// Returns empty Vec when the story can't be fetched — caller
    /// treats empty as "ambiguous category" and defaults up.
    fn get_story_labels(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Vec<String>> + Send;
    /// Fetch the most recent workflow run on main (#302 / #320).
    /// Returns `Ok(None)` when the repo has no runs yet. Errors
    /// surface as `Err(String)` to match the rest of the trait's
    /// Result-convention.
    fn list_latest_run_on_main(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<WorkflowRunEntry>, String>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalIssue {
    NotModified,
    Modified {
        value: serde_json::Value,
        etag: Option<String>,
    },
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalRunEntry {
    NotModified,
    /// #1287: now carries ALL recent runs (not just the most-recent
    /// single run) so the aggregate classifier can see every workflow
    /// triggered by the latest commit. Entries are ordered by GitHub's
    /// API default (most-recent first); the classifier filters by the
    /// first entry's head_sha to identify which runs belong to the
    /// latest commit.
    Modified {
        entries: Vec<WorkflowRunEntry>,
        etag: Option<String>,
    },
    Empty,
}
