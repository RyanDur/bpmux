use crate::ci::ConditionalRunEntry;
use crate::ci::slice::CiPollFetch;
use crate::integrations::github::tracker::OctocrabStoryTracker;

mod delegates;
mod ingest;
mod mutation;
mod mutation_events;
mod source_adapter;
pub(crate) mod tracker;

#[cfg(test)]
mod adapter_tests;
#[cfg(test)]
mod tracker_contract_tests;
#[cfg(test)]
mod tracker_tests;

pub struct GitHubSourceAdapter {
    inner: OctocrabStoryTracker,
}

impl CiPollFetch for GitHubSourceAdapter {
    fn list_latest_run_on_main_conditional<'a>(
        &'a self,
        prior_etag: Option<&'a str>,
    ) -> crate::ci::slice::BoxedFuture<'a, Result<ConditionalRunEntry, String>> {
        Box::pin(GitHubSourceAdapter::list_latest_run_on_main_conditional(
            self, prior_etag,
        ))
    }

    fn rate_limit_observer(&self) -> crate::rate_limit::RateLimitObserver {
        GitHubSourceAdapter::rate_limit_observer(self)
    }
}
