use super::Stories;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::board::item::ProductItem;
use crate::product_management::story::{StoryId, StoryKey};

pub(super) fn apply_raw_labels(w: &mut Stories, id: StoryId, fields: &LabelDerivedFields) {
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(story)) | Some(ProductItem::Iteration(story)) => {
            story.acceptor = fields.acceptor;
            story.parent_epic = fields.parent_epic;
            story.iteration_membership = fields.iteration_membership;
            story.tags = fields.tags.clone();
            story.ready_order = fields.ready_order;
            story.raw_labels = fields.raw_labels.clone();
        }
        Some(ProductItem::Outcome(outcome)) => {
            *outcome = outcome.with_horizon_labels(&fields.raw_labels);
        }
        None => {}
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, id: StoryId, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(existing)) | Some(ProductItem::Iteration(existing)) => {
            existing.acceptor = fields.acceptor;
            existing.parent_epic = fields.parent_epic;
            existing.iteration_membership = fields.iteration_membership;
            existing.tags = fields.tags.clone();
            existing.ready_order = fields.ready_order;
            existing.raw_labels = fields.raw_labels.clone();
        }
        Some(ProductItem::Outcome(outcome)) => {
            *outcome = outcome.with_horizon_labels(&fields.raw_labels);
        }
        None => {}
    }
}
