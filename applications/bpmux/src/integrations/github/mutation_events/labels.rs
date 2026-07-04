use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::integrations::github::mapper::is_position_label;
use crate::product_management::story::events::StoryEvent;
use crate::sync::SyncError;

pub(in crate::integrations::github) async fn push_tag_added(
    inner: &OctocrabStoryTracker,
    number: u64,
    label: &str,
) -> Result<Option<StoryEvent>, SyncError> {
    let labels = inner
        .add_label(number, label)
        .await
        .map_err(SyncError::Other)?;
    if is_position_label(label) {
        Ok(None)
    } else {
        Ok(Some(labels_refreshed(number, labels)))
    }
}

pub(in crate::integrations::github) async fn push_tag_removed(
    inner: &OctocrabStoryTracker,
    number: u64,
    label: &str,
) -> Result<Option<StoryEvent>, SyncError> {
    let labels = inner
        .remove_label(number, label)
        .await
        .map_err(SyncError::Other)?;
    if is_position_label(label) {
        Ok(None)
    } else {
        Ok(Some(labels_refreshed(number, labels)))
    }
}
