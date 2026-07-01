use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::product_management::board::adapters::github::encode_gate_comment;
use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::sync::SyncError;
use crate::product_management::story::Gate;

pub(in crate::integrations::github) async fn push_gate_comment_appended(
    inner: &OctocrabStoryTracker,
    number: u64,
    meta: &EventMeta,
    gate: Gate,
    payload: Option<&str>,
) -> Result<Option<StoryEvent>, SyncError> {
    let body = encode_gate_comment(gate, meta, payload);
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
