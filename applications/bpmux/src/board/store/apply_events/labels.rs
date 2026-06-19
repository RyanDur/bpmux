use super::Stories;
use crate::board::adapters::github;
use crate::board::store::label_sync::{
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
    }
}

pub(super) fn apply_labels_refreshed(w: &mut Stories, number: u64, labels: &[String]) {
    if labels.is_empty() {
    } else if let Some(existing) = w.get_mut(&number) {
        existing.state = github::state_from_labels(labels, existing.is_closed);
        existing.tags = labels
            .iter()
            .filter_map(|l| tag_from_label_string(l))
            .collect();
        existing.parent_epic = github::parent_epic_from_labels(labels);
        existing.iteration_membership = github::iteration_membership_from_labels(labels);
        existing.outcome_membership = github::outcome_membership_from_labels(labels);
        existing.iteration_order = github::iteration_order_from_labels(labels);
        existing.iteration_review_closed_by = github::iteration_review_closers_from_labels(labels);
        existing.iteration_dev_ok_by = github::iteration_dev_ok_by_from_labels(labels);
        existing.horizon = github::horizon_from_labels(labels);
        existing.is_current = github::is_current_from_labels(labels);
        existing.acceptor = github::acceptor_from_labels(labels);
        existing.raw_labels = labels.to_vec();
    }
}
