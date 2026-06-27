use super::Stories;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::story::StoryKey;

pub(super) fn apply_raw_labels(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(number)) {
        story.acceptor = fields.acceptor;
        story.parent_epic = fields.parent_epic;
        story.raw_labels = fields.raw_labels.clone();
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    if let Some(existing) = w.get_mut(&StoryKey::Saved(number)) {
        existing.acceptor = fields.acceptor;
        existing.parent_epic = fields.parent_epic;
        existing.raw_labels = fields.raw_labels.clone();
    }
}
