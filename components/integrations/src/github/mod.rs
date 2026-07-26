use crate::github::tracker::OctocrabStoryTracker;
use ::ci::ConditionalRunEntry;
use ::ci::slice::CiPollFetch;

pub mod connection;
mod delegates;
mod ingest;
pub mod keychain;
mod mutation;
mod mutation_events;
mod source_adapter;
pub mod tracker;

#[cfg(test)]
mod adapter_tests;
#[cfg(test)]
mod tracker_tests;

pub struct GitHubSourceAdapter {
    inner: OctocrabStoryTracker,
}

impl CiPollFetch for GitHubSourceAdapter {
    fn list_latest_run_on_main_conditional<'a>(
        &'a self,
        prior_etag: Option<&'a str>,
    ) -> ci::slice::BoxedFuture<'a, Result<ConditionalRunEntry, String>> {
        Box::pin(GitHubSourceAdapter::list_latest_run_on_main_conditional(
            self, prior_etag,
        ))
    }

    fn rate_limit_observer(&self) -> rate_limit::RateLimitObserver {
        GitHubSourceAdapter::rate_limit_observer(self)
    }
}
