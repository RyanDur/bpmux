use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::product_management::board::events::StoryEvent;
use crate::product_management::board::sync::SyncError;
use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;

pub(in crate::integrations::github) async fn push_parent_epic_set(
    inner: &OctocrabStoryTracker,
    number: u64,
    epic_id: EpicId,
) -> Result<Option<StoryEvent>, SyncError> {
    let label = format!("parent-epic-#{}", epic_id.value());
    let labels = inner
        .add_label(number, &label)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_parent_epic_unset(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let current = inner.get_story_labels(number).await;
    let mut last_response: Option<Vec<String>> = None;
    for l in current.iter().filter(|l| l.starts_with("parent-epic-#")) {
        let labels = inner
            .remove_label(number, l)
            .await
            .map_err(SyncError::Other)?;
        last_response = Some(labels);
    }
    Ok(last_response.map(|labels| labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_iteration_membership_set(
    inner: &OctocrabStoryTracker,
    number: u64,
    iteration_id: IterationId,
    position: u32,
) -> Result<Option<StoryEvent>, SyncError> {
    let current = inner.get_story_labels(number).await;
    for l in current
        .iter()
        .filter(|l| l.starts_with("parent-iteration-#"))
    {
        inner
            .remove_label(number, l)
            .await
            .map_err(SyncError::Other)?;
    }
    let label = format!("parent-iteration-#{}.{position}", iteration_id.value());
    let labels = inner
        .add_label(number, &label)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_iteration_membership_cleared(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let current = inner.get_story_labels(number).await;
    let mut last_response: Option<Vec<String>> = None;
    for l in current
        .iter()
        .filter(|l| l.starts_with("parent-iteration-#"))
    {
        let labels = inner
            .remove_label(number, l)
            .await
            .map_err(SyncError::Other)?;
        last_response = Some(labels);
    }
    Ok(last_response.map(|labels| labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_current_iteration_designated(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let labels = inner
        .add_label(
            number,
            crate::product_management::board::adapters::gh_label_codec::CURRENT_ITERATION_LABEL,
        )
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_current_iteration_cleared(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let labels = inner
        .remove_label(
            number,
            crate::product_management::board::adapters::gh_label_codec::CURRENT_ITERATION_LABEL,
        )
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}
