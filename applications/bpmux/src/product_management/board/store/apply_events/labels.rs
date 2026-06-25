use super::Stories;
use crate::product_management::board::events::LabelDerivedFields;

pub(super) fn apply_label_delta(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if let Some(story) = w.get_mut(&number) {
        story.tags = fields.tags.clone();
        story.raw_labels = fields.raw_labels.clone();
        story.parent_epic = fields.parent_epic;
        story.iteration_membership = fields.iteration_membership;
        story.iteration_order = fields.iteration_order;
        story.iteration_review_closed_by = fields.iteration_review_closed_by.clone();
        story.iteration_dev_ok_by = fields.iteration_dev_ok_by.clone();
        story.horizon = fields.horizon;
        story.is_current = fields.is_current;
        story.acceptor = fields.acceptor;
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, number: u64, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    if let Some(existing) = w.get_mut(&number) {
        existing.tags = fields.tags.clone();
        existing.parent_epic = fields.parent_epic;
        existing.iteration_membership = fields.iteration_membership;
        existing.outcome_membership = fields.outcome_membership;
        existing.iteration_order = fields.iteration_order;
        existing.iteration_review_closed_by = fields.iteration_review_closed_by.clone();
        existing.iteration_dev_ok_by = fields.iteration_dev_ok_by.clone();
        existing.horizon = fields.horizon;
        existing.is_current = fields.is_current;
        existing.acceptor = fields.acceptor;
        existing.raw_labels = fields.raw_labels.clone();
    }
}
