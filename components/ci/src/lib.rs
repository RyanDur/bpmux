pub mod data_gateway;
pub mod db;
use std::time::Duration;

mod actions_url;
#[cfg(test)]
mod actions_url_tests;
mod broadcast;
mod classify;
mod jobs;
mod log_parse;
#[cfg(test)]
mod log_parse_tests;
mod log_store;
#[cfg(test)]
mod log_store_tests;
mod runs;
mod state_report;

pub use actions_url::{ActionsUrl, GITHUB_BASE_URL};
pub use broadcast::{
    build_dev_ci_signal, build_multi_depth_for_transition, compute_broadcast_text,
};
pub use classify::{RunEntry, classify_runs};
pub use jobs::{CiJobRecord, CiJobStatusError, compute_duration_ms};
pub use log_parse::{build_non_test_failure_record, parse_playwright_failures_from_log};
pub use log_store::CiLogStore;
pub use runs::{CiLogError, CiRunFailures, CiTestFailure, ConditionalRunEntry, WorkflowRunEntry};
pub(crate) use state_report::report_from_event;
pub use state_report::{CiStateReport, unknown_ci_state_report};
pub mod slice;

pub const POLL_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_BACKOFF: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiState {
    #[default]
    Unknown,
    Green,
    Red,
    Running,
    QuotaExhausted,
}

impl From<CiState> for message::CiStateWire {
    fn from(state: CiState) -> Self {
        use message::CiStateWire;
        match state {
            CiState::Unknown => CiStateWire::Unknown,
            CiState::Green => CiStateWire::Green,
            CiState::Red => CiStateWire::Red,
            CiState::Running => CiStateWire::Running,
            CiState::QuotaExhausted => CiStateWire::QuotaExhausted,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CiEvent {
    pub state: CiState,
    pub sha: Option<String>,
    pub title: Option<String>,
    pub run_id: Option<u64>,
    pub workflows: Vec<WorkflowSummary>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WorkflowSummary {
    pub name: Option<String>,
    pub run_id: Option<u64>,
    pub conclusion: &'static str,
}
