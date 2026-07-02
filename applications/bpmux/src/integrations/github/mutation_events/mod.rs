use crate::product_management::board::events::StoryEvent;

mod acceptor;
mod blocked;
mod comments;
mod grouping;
mod labels;
mod lifecycle;

pub(in crate::integrations::github) use acceptor::push_acceptor_assigned;
pub(in crate::integrations::github) use blocked::{push_blocked_by_cleared, push_blocked_by_set};
pub(in crate::integrations::github) use comments::{
    push_free_comment_appended, push_gate_comment_appended,
};
pub(in crate::integrations::github) use grouping::{
    push_current_iteration_cleared, push_current_iteration_designated, push_parent_epic_set,
    push_parent_epic_unset,
};
pub(in crate::integrations::github) use labels::{push_tag_added, push_tag_removed};
pub(in crate::integrations::github) use lifecycle::{
    push_state_transitioned, push_story_closed, push_story_reopened,
};

pub(super) fn labels_refreshed(number: u64, labels: Vec<String>) -> StoryEvent {
    StoryEvent::LabelsRefreshed {
        meta: crate::product_management::board::orchestrator::cache_fill_meta(),
        id: crate::product_management::story::StoryId::new(number),
        fields: crate::product_management::board::adapters::github::project_labels(&labels),
    }
}
