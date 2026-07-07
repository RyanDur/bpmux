use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::{decode_story_metadata, outbound_state_label};
use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::product_management::story::{StoryId, Work};
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
    new_state: Work,
    member_of_current_iteration: bool,
) -> Result<Option<StoryEvent>, SyncError> {
    let Some(keep) = outbound_state_label(&new_state, member_of_current_iteration) else {
        strip_state_labels(inner, number, "").await?;
        let labels = inner.get_story_labels(number).await;
        return Ok(Some(labels_refreshed(number, labels)));
    };
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
    strip_state_labels(
        inner,
        number,
        crate::product_management::scheduling::ProjectedState::Accepted.github_label(),
    )
    .await?;
    match inner.close_story(number).await {
        Ok(value) => Ok(decode_story_metadata(&value).map(|story| {
            StoryEvent::new(
                crate::sync::cache_fill_meta(),
                StoryId::new(number),
                StoryEventKind::StoryRefreshed {
                    story: Box::new(story),
                },
            )
        })),
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
    Ok(decode_story_metadata(&value).map(|story| {
        StoryEvent::new(
            crate::sync::cache_fill_meta(),
            StoryId::new(number),
            StoryEventKind::StoryRefreshed {
                story: Box::new(story),
            },
        )
    }))
}

pub(in crate::integrations::github) async fn push_body_updated(
    inner: &OctocrabStoryTracker,
    number: u64,
    body: &str,
) -> Result<Option<StoryEvent>, SyncError> {
    inner
        .update_body(number, body)
        .await
        .map_err(SyncError::Other)?;
    Ok(None)
}

pub(in crate::integrations::github) async fn push_title_updated(
    inner: &OctocrabStoryTracker,
    number: u64,
    title: &str,
) -> Result<Option<StoryEvent>, SyncError> {
    inner
        .update_title(number, title)
        .await
        .map_err(SyncError::Other)?;
    Ok(None)
}
