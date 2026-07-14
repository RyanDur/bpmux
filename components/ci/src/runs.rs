#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiTestFailure {
    pub shard_id: Option<String>,
    pub test_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub error_message: String,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalRunEntry {
    NotModified,
    Modified {
        entries: Vec<WorkflowRunEntry>,
        etag: Option<String>,
    },
    Empty,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CiRunFailures {
    pub run_id: u64,
    pub conclusion: String,
    pub failures: Vec<CiTestFailure>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CiLogError {
    RunNotFound { run_id: u64 },
    RunInProgress { run_id: u64, conclusion: String },
    RateLimited { wait_secs: u64 },
    UpstreamError { detail: String },
}
