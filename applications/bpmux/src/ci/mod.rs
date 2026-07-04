mod broadcast;
mod classify;
mod state_report;
#[cfg(test)]
mod state_report_tests;
mod state_store;
#[cfg(test)]
mod state_store_tests;

#[cfg(test)]
pub(crate) use crate::ci_link::resolve_ci_actions_url;
pub use crate::ci_link::{GITHUB_BASE_URL, github_actions_url};
pub(crate) use broadcast::{
    build_dev_ci_signal, build_multi_depth_for_transition, compute_broadcast_text,
};
pub use classify::{RunEntry, classify_runs};
pub(crate) use state_report::{CiStateReport, build_ci_state_report, unknown_ci_state_report};
pub use state_store::CiStateStore;
pub mod slice;

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

impl From<CiState> for crate::message::CiStateWire {
    fn from(state: CiState) -> Self {
        use crate::message::CiStateWire;
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
