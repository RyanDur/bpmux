use std::future::Future;
use std::pin::Pin;

use crate::{CiEvent, ConditionalRunEntry};
use rate_limit::RateLimitObserver;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait CiPollFetch: Send + Sync + 'static {
    fn list_latest_run_on_main_conditional<'a>(
        &'a self,
        prior_etag: Option<&'a str>,
    ) -> BoxedFuture<'a, Result<ConditionalRunEntry, String>>;

    fn rate_limit_observer(&self) -> RateLimitObserver;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiSnapshot {
    pub state: crate::CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    pub conclusion: &'static str,
    pub workflows: Vec<crate::WorkflowSummary>,
}

impl CiSnapshot {
    pub fn to_ci_event(&self) -> CiEvent {
        CiEvent {
            state: self.state,
            sha: self.head_sha.clone(),
            title: self.display_title.clone(),
            run_id: self.run_id,
            workflows: self.workflows.clone(),
        }
    }
}

pub trait CiBroadcastListener: Send + Sync {
    fn on_changed(&self, snap: &CiSnapshot);
}

#[derive(Default, Clone)]
pub struct CiState {
    last_seen: Option<(Option<u64>, &'static str)>,
    snapshot: Option<CiSnapshot>,
    polling: bool,
    last_successful_poll_at: Option<String>,
    last_updated_at: Option<String>,
}

pub fn reduce(state: &CiState, action: &CiAction) -> Option<CiState> {
    match action {
        CiAction::Updated(snap) => {
            let key = (snap.run_id, snap.conclusion);
            if state.last_seen == Some(key) {
                return None;
            }
            Some(CiState {
                last_seen: Some(key),
                snapshot: Some(snap.clone()),
                ..state.clone()
            })
        }
        CiAction::PollingStarted => Some(CiState {
            polling: true,
            ..state.clone()
        }),
        CiAction::PollingStopped => Some(CiState {
            polling: false,
            ..state.clone()
        }),
        CiAction::StartPolling | CiAction::StopPolling => None,
        CiAction::PollSucceeded { at, updated } => Some(CiState {
            last_successful_poll_at: Some(at.clone()),
            last_updated_at: if *updated {
                Some(at.clone())
            } else {
                state.last_updated_at.clone()
            },
            ..state.clone()
        }),
    }
}

pub fn snapshot(state: &CiState) -> Option<CiSnapshot> {
    state.snapshot.clone()
}

pub fn polling(state: &CiState) -> bool {
    state.polling
}

pub fn snapshot_key(state: &CiState) -> Option<(Option<u64>, &'static str)> {
    state
        .snapshot
        .as_ref()
        .map(|snap| (snap.run_id, snap.conclusion))
}

pub fn last_successful_poll_at(state: &CiState) -> Option<String> {
    state.last_successful_poll_at.clone()
}

pub fn last_updated_at(state: &CiState) -> Option<String> {
    state.last_updated_at.clone()
}

pub fn report_from_state(state: &CiState) -> crate::CiStateReport {
    match &state.snapshot {
        None => crate::unknown_ci_state_report(),
        Some(snap) => {
            let mut report = crate::report_from_event(snap.to_ci_event());
            report.last_successful_poll_at = state.last_successful_poll_at.clone();
            report.last_updated_at = state.last_updated_at.clone();
            report
        }
    }
}

mod action;

#[cfg(test)]
mod reduce_tests;
#[cfg(test)]
mod report_from_state_tests;

pub use action::CiAction;
