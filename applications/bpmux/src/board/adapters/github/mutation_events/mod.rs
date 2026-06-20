use crate::board::events::BoardEvent;

mod comments;
mod grouping;
mod labels;
mod lifecycle;

pub(in crate::board::adapters::github) use comments::{
    push_free_comment_appended, push_gate_comment_appended,
};
pub(in crate::board::adapters::github) use grouping::{
    push_current_iteration_cleared, push_current_iteration_designated,
    push_iteration_membership_cleared, push_iteration_membership_set, push_outcome_membership_set,
    push_parent_epic_set, push_parent_epic_unset,
};
pub(in crate::board::adapters::github) use labels::{push_tag_added, push_tag_removed};
pub(in crate::board::adapters::github) use lifecycle::{
    push_state_transitioned, push_story_closed, push_story_reopened,
};

pub(super) fn labels_refreshed(number: u64, labels: Vec<String>) -> BoardEvent {
    BoardEvent::LabelsRefreshed {
        meta: crate::board::orchestrator::cache_fill_meta(),
        number,
        labels,
    }
}
