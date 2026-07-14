use super::labels_refreshed;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::tracker::{OctocrabStoryTracker, StoryTracker};
use crate::sync::SyncError;
use product_management::label_codec::LabelCodec;
use product_management::story::Acceptor;
use product_management::story::events::StoryEvent;

pub(in crate::integrations::github) async fn push_acceptor_assigned(
    inner: &OctocrabStoryTracker,
    number: u64,
    acceptor: Acceptor,
) -> Result<Option<StoryEvent>, SyncError> {
    let keep = GhLabelCodec.encode_acceptor(acceptor);
    let current = inner.get_story_labels(number).await;
    for stale in current
        .iter()
        .filter(|l| GhLabelCodec.decode_acceptor(l).is_some() && l.as_str() != keep)
    {
        inner
            .remove_label(number, stale)
            .await
            .map_err(SyncError::Other)?;
    }
    let labels = inner
        .add_label(number, keep)
        .await
        .map_err(SyncError::Other)?;
    Ok(Some(labels_refreshed(number, labels)))
}
