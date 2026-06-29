use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::product_management::board::adapters::github::{decode_story_metadata, state_to_label};
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::sync::SyncError;
use crate::product_management::story::{StoryId, StoryState};

pub(in crate::integrations::github) async fn push_state_transitioned(
    inner: &OctocrabStoryTracker,
    number: u64,
    new_state: StoryState,
) -> Result<Option<BoardEvent>, SyncError> {
    let label = state_to_label(new_state);
    let labels = inner
        .add_label(number, label)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_story_closed(
    inner: &OctocrabStoryTracker,
    number: u64,
    reason: &Option<String>,
) -> Result<Option<BoardEvent>, SyncError> {
    if let Some(reason_text) = reason.as_deref() {
        inner
            .comment_on_story(number, reason_text)
            .await
            .map_err(SyncError::Other)?;
    }
    match inner.close_story(number).await {
        Ok(value) => Ok(
            decode_story_metadata(&value).map(|story| BoardEvent::StoryRefreshed {
                meta: crate::product_management::board::orchestrator::cache_fill_meta(),
                id: StoryId::new(number),
                story: Box::new(story),
            }),
        ),
        Err(e) => match crate::product_management::board::sync::classify_close_error(e) {
            SyncError::AlreadyClosed => Ok(None),
            other => Err(other),
        },
    }
}

pub(in crate::integrations::github) async fn push_story_reopened(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<BoardEvent>, SyncError> {
    let value = inner.reopen_story(number).await.map_err(SyncError::Other)?;
    Ok(
        decode_story_metadata(&value).map(|story| BoardEvent::StoryRefreshed {
            meta: crate::product_management::board::orchestrator::cache_fill_meta(),
            id: StoryId::new(number),
            story: Box::new(story),
        }),
    )
}
