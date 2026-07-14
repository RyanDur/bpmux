use product_management::story::events::{StoryEvent, StoryEventKind};

mod acceptor;
mod comments;
mod grouping;
mod labels;
mod lifecycle;

pub(in crate::integrations::github) use acceptor::push_acceptor_assigned;
pub(in crate::integrations::github) use comments::{push_free_comment_appended, push_verdict_cast};
pub(in crate::integrations::github) use grouping::{
    push_current_iteration_cleared, push_current_iteration_designated, push_parent_epic_set,
    push_parent_epic_unset,
};
pub(in crate::integrations::github) use labels::{push_tag_added, push_tag_removed};
pub(in crate::integrations::github) use lifecycle::{
    push_body_updated, push_state_transitioned, push_story_closed, push_story_reopened,
    push_title_updated,
};

pub(super) fn labels_refreshed(number: u64, labels: Vec<String>) -> StoryEvent {
    StoryEvent::new(
        crate::sync::cache_fill_meta(),
        product_management::story::StoryId::new(number),
        StoryEventKind::LabelsRefreshed {
            fields: source_codec::mapper::project_labels(&labels),
        },
    )
}
