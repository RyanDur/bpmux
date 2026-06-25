use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};

mod comments;
mod grouping;
mod labels;
mod lifecycle;

pub(in crate::integrations::github) use comments::{
    push_free_comment_appended, push_gate_comment_appended,
};
pub(in crate::integrations::github) use grouping::{
    push_current_iteration_cleared, push_current_iteration_designated,
    push_iteration_membership_cleared, push_iteration_membership_set,
    push_outcome_membership_cleared, push_outcome_membership_set, push_parent_epic_set,
    push_parent_epic_unset,
};
pub(in crate::integrations::github) use labels::{push_tag_added, push_tag_removed};
pub(in crate::integrations::github) use lifecycle::{
    push_state_transitioned, push_story_closed, push_story_reopened,
};

pub(super) fn labels_refreshed(number: u64, labels: Vec<String>) -> BoardEvent {
    use crate::product_management::board::adapters::github;
    let fields = LabelDerivedFields {
        tags: github::tags_from_labels(&labels),
        parent_epic: github::parent_epic_from_labels(&labels),
        iteration_membership: github::iteration_membership_from_labels(&labels),
        outcome_membership: github::outcome_membership_from_labels(&labels),
        iteration_order: github::iteration_order_from_labels(&labels),
        iteration_review_closed_by: github::iteration_review_closers_from_labels(&labels),
        iteration_dev_ok_by: github::iteration_dev_ok_by_from_labels(&labels),
        horizon: github::horizon_from_labels(&labels),
        horizon_order: github::horizon_order_from_labels(&labels),
        is_current: github::is_current_from_labels(&labels),
        acceptor: github::acceptor_from_labels(&labels),
        raw_labels: labels,
    };
    BoardEvent::LabelsRefreshed {
        meta: crate::product_management::board::orchestrator::cache_fill_meta(),
        number,
        fields,
    }
}
