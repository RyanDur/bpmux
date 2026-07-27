use super::*;
use crate::github::tracker::{OctocrabStoryTracker, StoryTracker};

impl GitHubSourceAdapter {
    pub fn new(inner: OctocrabStoryTracker) -> Self {
        Self { inner }
    }

    pub fn from_connection(
        connection: &crate::github::connection::GithubConnection,
        observer: rate_limit::RateLimitObserver,
    ) -> Option<Self> {
        OctocrabStoryTracker::from_connection(connection, observer).map(Self::new)
    }

    pub async fn accept_story(&self, number: u64) -> Result<(), String> {
        self.inner.accept_story(number).await
    }

    pub async fn reject_story(&self, number: u64, note: &str) -> Result<(), String> {
        self.inner.reject_story(number, note).await
    }

    pub async fn post_story_comment(&self, number: u64, body: &str) -> Result<(), String> {
        self.inner.post_story_comment(number, body).await
    }

    pub async fn list_story_timeline(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        self.inner.list_story_timeline(number).await
    }

    pub async fn list_latest_run_on_main(&self) -> Result<Option<ci::WorkflowRunEntry>, String> {
        self.inner.list_latest_run_on_main().await
    }

    pub async fn list_latest_run_on_main_conditional(
        &self,
        prior_etag: Option<&str>,
    ) -> Result<ci::ConditionalRunEntry, String> {
        self.inner
            .list_latest_run_on_main_conditional(prior_etag)
            .await
    }

    pub async fn fetch_failed_tests_for_run(
        &self,
        run_id: u64,
    ) -> Result<ci::CiRunFailures, ci::CiLogError> {
        self.inner.fetch_failed_tests_for_run(run_id).await
    }

    pub async fn rerun_failed_jobs(&self, run_id: u64) -> Result<(), String> {
        self.inner.rerun_failed_jobs(run_id).await
    }

    pub async fn fetch_job_status_for_run(
        &self,
        run_id: u64,
    ) -> Result<Vec<ci::CiJobRecord>, ci::CiJobStatusError> {
        self.inner.fetch_job_status_for_run(run_id).await
    }

    pub fn rate_limit_observer(&self) -> rate_limit::RateLimitObserver {
        self.inner.rate_limit_observer()
    }

    pub async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
        self.inner.get_story(number).await
    }

    pub async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        self.inner.fetch_board_stories().await
    }

    pub async fn list_story_comments(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        self.inner.list_story_comments(number).await
    }
}
