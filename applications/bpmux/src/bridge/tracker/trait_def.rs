pub(crate) use crate::github::ensure_crypto_provider_installed;

pub(crate) fn log_labels_fetch_failure<E: std::fmt::Display>(
    backend: &str,
    number: u64,
    err: E,
) -> Vec<String> {
    crate::diag::log_diag(&format!(
        "get_story_labels ({backend}) #{number} failed: {err}"
    ));
    Vec::new()
}

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
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

pub trait StoryTracker: Clone + Send + Sync + 'static {
    fn close_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn reopen_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn add_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    fn remove_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn update_title(
        &self,
        number: u64,
        title: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn fetch_board_stories(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send;
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
    fn get_story_labels(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Vec<String>> + Send;
    fn list_latest_run_on_main(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<WorkflowRunEntry>, String>> + Send;
    #[allow(dead_code)]
    fn list_sub_issues(
        &self,
        parent: u64,
    ) -> impl std::future::Future<Output = Result<Vec<u64>, String>> + Send;
    #[allow(dead_code)]
    fn add_sub_issue(
        &self,
        parent: u64,
        child: u64,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
    #[allow(dead_code)]
    fn remove_sub_issue(
        &self,
        parent: u64,
        child: u64,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
    #[allow(dead_code)]
    fn reprioritize_sub_issue(
        &self,
        parent: u64,
        child: u64,
        after: Option<u64>,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
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
    Modified {
        entries: Vec<WorkflowRunEntry>,
        etag: Option<String>,
    },
    Empty,
}
