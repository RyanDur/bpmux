use super::labels_refreshed;
use crate::board::adapters::github::is_position_label;
use crate::board::events::BoardEvent;
use crate::board::sync::SyncError;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};

pub(in crate::board::adapters::github) async fn push_tag_added(
    inner: &OctocrabStoryTracker,
    number: u64,
    label: &str,
) -> Result<Option<BoardEvent>, SyncError> {
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

pub(in crate::board::adapters::github) async fn push_tag_removed(
    inner: &OctocrabStoryTracker,
    number: u64,
    label: &str,
) -> Result<Option<BoardEvent>, SyncError> {
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
