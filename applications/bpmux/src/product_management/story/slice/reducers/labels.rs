use super::Stories;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::story::{StoryId, StoryKey};

pub(super) fn apply_raw_labels(w: &mut Stories, id: StoryId, fields: &LabelDerivedFields) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(id)) {
        story.iteration_membership = fields.iteration_membership;
        story.tags = fields.tags.clone();
        story.raw_labels = fields.raw_labels.clone();
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, id: StoryId, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    if let Some(existing) = w.get_mut(&StoryKey::Saved(id)) {
        existing.acceptor = fields.acceptor;
        existing.parent_epic = fields.parent_epic;
        existing.iteration_membership = fields.iteration_membership;
        existing.tags = fields.tags.clone();
        existing.ready_order = fields.ready_order;
        existing.raw_labels = fields.raw_labels.clone();
    }
}
