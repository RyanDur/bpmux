use serde::Serialize;

use crate::ci::{CiJobRecord, CiJobStatusError};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobTruncation {
    pub shown: usize,
    pub total: usize,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobStatusData {
    pub run_id: u64,
    pub jobs: Vec<CiJobRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<CiJobTruncation>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CiJobStatusResponse {
    pub mutation_status: &'static str,
    pub data: CiJobStatusData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action_hint: Option<String>,
}

pub(super) fn serialize_error(err: &CiJobStatusError) -> String {
    serde_json::to_string(err).unwrap_or_else(|_| "{}".to_string())
}
