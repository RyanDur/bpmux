use super::Stories;
use crate::product_management::board::events::LabelDerivedFields;

pub(super) fn apply_label_delta(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if let Some(story) = w.get_mut(&number) {
        story.raw_labels = fields.raw_labels.clone();
        story.parent_epic = fields.parent_epic;
    }
}

pub(super) fn apply_raw_labels_only(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if let Some(story) = w.get_mut(&number) {
        story.raw_labels = fields.raw_labels.clone();
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    if let Some(existing) = w.get_mut(&number) {
        existing.parent_epic = fields.parent_epic;
        existing.raw_labels = fields.raw_labels.clone();
    }
}
