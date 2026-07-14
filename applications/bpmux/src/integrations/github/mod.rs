use crate::bridge::tracker::OctocrabStoryTracker;
use crate::ci::ConditionalRunEntry;
use crate::ci::slice::CiPollFetch;

mod delegates;
pub mod gh_label_codec;
mod ingest;
pub mod mapper;
mod mutation;
mod mutation_events;
mod relationship_block;
pub mod relationships;
mod source_adapter;

#[cfg(test)]
mod adapter_tests;
#[cfg(test)]
mod relationship_block_tests;
#[cfg(test)]
mod relationships_tests;

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
