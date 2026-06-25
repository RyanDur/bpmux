use std::future::Future;
use std::pin::Pin;

use crate::bridge::tracker::ConditionalRunEntry;
use crate::ci::{CiEvent, CiState};
use crate::github::RateLimitObserver;

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
    pub state: CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
    pub conclusion: &'static str,
    pub workflows: Vec<crate::ci::WorkflowSummary>,
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

mod store;

pub use store::{CiReduceOutcome, CiStore};
