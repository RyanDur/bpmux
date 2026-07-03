mod broadcast;
mod classify;

#[cfg(test)]
pub(crate) use crate::ci_link::resolve_ci_actions_url;
pub use crate::ci_link::{GITHUB_BASE_URL, github_actions_url};
pub(crate) use broadcast::{
    build_dev_ci_signal, build_multi_depth_for_transition, compute_broadcast_text,
};
pub use classify::{RunEntry, classify_runs};
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
