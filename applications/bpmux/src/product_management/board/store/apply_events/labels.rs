use super::Stories;
use crate::product_management::board::adapters::github;
use crate::product_management::board::events::LabelDerivedFields;
use crate::product_management::board::store::label_sync::{
    sync_raw_labels_on_tag_added, sync_raw_labels_on_tag_removed, tag_from_label_string,
};

pub(super) fn apply_tag_added(w: &mut Stories, number: u64, label: &str) {
    if let Some(story) = w.get_mut(&number) {
        if let Some(tag) = tag_from_label_string(label)
            && !story.tags.contains(&tag)
        {
            story.tags.push(tag);
        }
        sync_raw_labels_on_tag_added(story, label);
        story.iteration_order = github::iteration_order_from_labels(&story.raw_labels);
        story.iteration_review_closed_by =
            github::iteration_review_closers_from_labels(&story.raw_labels);
        story.iteration_dev_ok_by = github::iteration_dev_ok_by_from_labels(&story.raw_labels);
        story.horizon = github::horizon_from_labels(&story.raw_labels);
        story.horizon_order = github::horizon_order_from_labels(&story.raw_labels);
    }
}

pub(super) fn apply_tag_removed(w: &mut Stories, number: u64, label: &str) {
    if let Some(story) = w.get_mut(&number) {
        if let Some(tag) = tag_from_label_string(label) {
            story.tags.retain(|t| *t != tag);
        }
        sync_raw_labels_on_tag_removed(story, label);
        story.iteration_order = github::iteration_order_from_labels(&story.raw_labels);
        story.iteration_review_closed_by =
            github::iteration_review_closers_from_labels(&story.raw_labels);
        story.iteration_dev_ok_by = github::iteration_dev_ok_by_from_labels(&story.raw_labels);
        story.horizon = github::horizon_from_labels(&story.raw_labels);
        story.horizon_order = github::horizon_order_from_labels(&story.raw_labels);
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
        existing.horizon_order = fields.horizon_order;
        existing.is_current = fields.is_current;
        existing.acceptor = fields.acceptor;
        existing.raw_labels = fields.raw_labels.clone();
    }
}
