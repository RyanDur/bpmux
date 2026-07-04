use super::labels_refreshed;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::board::sync::SyncError;
use crate::product_management::story::events::StoryEvent;

pub(in crate::integrations::github) async fn push_iteration_slot_set(
    inner: &OctocrabStoryTracker,
    number: u64,
    name: &str,
    position: u32,
) -> Result<Option<StoryEvent>, SyncError> {
    let keep = GhLabelCodec.encode_iteration(name, position);
    let current = inner.get_story_labels(number).await;
    for stale in current
        .iter()
        .filter(|l| GhLabelCodec.is_iteration_label_loose(l) && l.as_str() != keep)
    {
        inner
            .remove_label(number, stale)
            .await
            .map_err(SyncError::Other)?;
    }
    let labels = inner
        .add_label(number, &keep)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}

pub(in crate::integrations::github) async fn push_iteration_slot_cleared(
    inner: &OctocrabStoryTracker,
    number: u64,
) -> Result<Option<StoryEvent>, SyncError> {
    let current = inner.get_story_labels(number).await;
    let mut labels = current.clone();
    for stale in current
        .iter()
        .filter(|l| GhLabelCodec.is_iteration_label_loose(l))
    {
        labels = inner
            .remove_label(number, stale)
            .await
            .map_err(SyncError::Other)?;
    }
    Ok(Some(labels_refreshed(number, labels)))
}
