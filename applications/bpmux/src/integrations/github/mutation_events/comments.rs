use crate::integrations::github::tracker::{OctocrabStoryTracker, StoryTracker};
use product_management::story::Verdict;
use product_management::story::events::{EventMeta, StoryEvent};
use source_codec::mapper::encode_verdict;
use source_sync::SyncError;

pub(in crate::integrations::github) async fn push_verdict_cast(
    inner: &OctocrabStoryTracker,
    number: u64,
    meta: &EventMeta,
    verdict: &Verdict,
    payload: Option<&str>,
) -> Result<Option<StoryEvent>, SyncError> {
    let body = encode_verdict(verdict, meta, payload);
    inner
        .comment_on_story(number, &body)
        .await
        .map_err(SyncError::Other)?;
    Ok(None)
}

pub(in crate::integrations::github) async fn push_free_comment_appended(
    inner: &OctocrabStoryTracker,
    number: u64,
    body: &str,
) -> Result<Option<StoryEvent>, SyncError> {
    inner
        .comment_on_story(number, body)
        .await
        .map_err(SyncError::Other)?;
    Ok(None)
}
