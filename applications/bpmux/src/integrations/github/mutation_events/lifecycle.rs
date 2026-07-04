use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::{decode_story_metadata, state_to_label};
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{BoardState, StoryId};
use crate::sync::SyncError;

async fn strip_state_labels(
    inner: &OctocrabStoryTracker,
    number: u64,
    keep: &str,
) -> Result<(), SyncError> {
    let current = inner.get_story_labels(number).await;
    for stale in current
        .iter()
        .filter(|l| GhLabelCodec.is_workflow_state_label(l) && !l.eq_ignore_ascii_case(keep))
    {
        inner
            .remove_label(number, stale)
            .await
            .map_err(SyncError::Other)?;
    }
    Ok(())
}

pub(in crate::integrations::github) async fn push_state_transitioned(
    inner: &OctocrabStoryTracker,
    number: u64,
    new_state: BoardState,
) -> Result<Option<StoryEvent>, SyncError> {
    let keep = state_to_label(new_state);
    strip_state_labels(inner, number, keep).await?;
    let labels = inner
        .add_label(number, keep)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_story_closed(
    inner: &OctocrabStoryTracker,
    number: u64,
    reason: &Option<String>,
) -> Result<Option<StoryEvent>, SyncError> {
    if let Some(reason_text) = reason.as_deref() {
        inner
            .comment_on_story(number, reason_text)
            .await
            .map_err(SyncError::Other)?;
    }
    strip_state_labels(inner, number, BoardState::Accepted.github_label()).await?;
    match inner.close_story(number).await {
        Ok(value) => Ok(
            decode_story_metadata(&value).map(|story| StoryEvent::StoryRefreshed {
                meta: crate::sync::cache_fill_meta(),
                id: StoryId::new(number),
                story: Box::new(story),
            }),
        ),
        Err(e) => match crate::sync::classify_close_error(e) {
            SyncError::AlreadyClosed => Ok(None),
            other => Err(other),
        },
    }
}

pub(in crate::integrations::github) async fn push_story_reopened(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let value = inner.reopen_story(number).await.map_err(SyncError::Other)?;
    Ok(
        decode_story_metadata(&value).map(|story| StoryEvent::StoryRefreshed {
            meta: crate::sync::cache_fill_meta(),
            id: StoryId::new(number),
            story: Box::new(story),
        }),
    )
}
