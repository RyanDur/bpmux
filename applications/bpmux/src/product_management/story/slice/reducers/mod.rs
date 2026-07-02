use std::collections::HashMap;

use super::action::StoryAction;
use crate::product_management::board::events::StoryEvent;
use crate::product_management::story::{Story, StoryId, StoryKey};

mod comments;
mod labels;
mod lifecycle;

use comments::{
    apply_free_comment_appended, apply_free_comments_fetched, apply_gate_comment_appended,
    apply_gate_comments_fetched,
};
use labels::{apply_labels_refreshed, apply_raw_labels};
use lifecycle::{
    apply_acceptor_assigned, apply_blocked_by_cleared, apply_blocked_by_set, apply_body_fetched,
    apply_body_updated, apply_iteration_slot_cleared, apply_iteration_slot_set,
    apply_parent_epic_set, apply_parent_epic_unset, apply_state_transitioned, apply_story_closed,
    apply_story_created, apply_story_refreshed, apply_story_reopened, apply_tag_set,
    apply_tag_unset, apply_title_updated,
};

pub(crate) type Stories = HashMap<StoryKey, Story>;

pub(super) fn story_mut(w: &mut Stories, id: StoryId) -> Option<&mut Story> {
    w.get_mut(&StoryKey::Saved(id))
}

pub fn reduce(state: &Stories, action: &StoryAction) -> Stories {
    let mut stories = state.clone();
    match action {
        StoryAction::Event(event) => apply_event(&mut stories, event),
        StoryAction::RawLabelsAssigned { id, fields } => {
            apply_raw_labels(&mut stories, *id, fields)
        }
        StoryAction::Seeded(incoming) => {
            for item in incoming {
                stories.insert(item.key(), item.clone());
            }
        }
        StoryAction::Removed(id) => {
            stories.remove(&StoryKey::Saved(*id));
        }
        StoryAction::Cleared => stories.clear(),
    }
    stories
}

fn apply_event(w: &mut Stories, event: &StoryEvent) {
    let id = event.story_id();
    match event {
        StoryEvent::StoryCreated {
            meta,
            id: n,
            title,
            body,
            initial_state,
            story_type,
        } => apply_story_created(w, *n, meta, title, body, *initial_state, *story_type),
        StoryEvent::StateTransitioned { new_state, .. } => {
            apply_state_transitioned(w, id, *new_state)
        }
        StoryEvent::GateCommentAppended {
            meta,
            gate,
            payload,
            ..
        } => apply_gate_comment_appended(w, id, meta, *gate, payload.clone()),
        StoryEvent::FreeCommentAppended { meta, body, .. } => {
            apply_free_comment_appended(w, id, meta, body)
        }
        StoryEvent::BodyFetched { body, etag, .. } => apply_body_fetched(w, id, body, etag),
        StoryEvent::FreeCommentsFetched { comments, .. } => {
            apply_free_comments_fetched(w, id, comments)
        }
        StoryEvent::GateCommentsFetched { comments, .. } => {
            apply_gate_comments_fetched(w, id, comments)
        }
        StoryEvent::StoryClosed { .. } => apply_story_closed(w, id),
        StoryEvent::StoryReopened { .. } => apply_story_reopened(w, id),
        StoryEvent::AcceptorAssigned { acceptor, .. } => apply_acceptor_assigned(w, id, *acceptor),
        StoryEvent::ParentEpicSet { epic_id, .. } => apply_parent_epic_set(w, id, *epic_id),
        StoryEvent::ParentEpicUnset { .. } => apply_parent_epic_unset(w, id),
        StoryEvent::TagSet { tag, .. } => apply_tag_set(w, id, *tag),
        StoryEvent::TagUnset { tag, .. } => apply_tag_unset(w, id, *tag),
        StoryEvent::BlockedBySet { blocker, .. } => apply_blocked_by_set(w, id, *blocker),
        StoryEvent::BlockedByCleared { .. } => apply_blocked_by_cleared(w, id),
        StoryEvent::IterationSlotSet { name, position, .. } => {
            apply_iteration_slot_set(w, id, name, *position)
        }
        StoryEvent::IterationSlotCleared { .. } => apply_iteration_slot_cleared(w, id),
        StoryEvent::TagAdded { .. }
        | StoryEvent::TagRemoved { .. }
        | StoryEvent::CurrentIterationDesignated { .. }
        | StoryEvent::CurrentIterationCleared { .. } => {}
        StoryEvent::StoryRefreshed { story, .. } => apply_story_refreshed(w, id, story),
        StoryEvent::TitleUpdated { title, .. } => apply_title_updated(w, id, title),
        StoryEvent::BodyUpdated { body, .. } => apply_body_updated(w, id, body),
        StoryEvent::LabelsRefreshed { fields, .. } => apply_labels_refreshed(w, id, fields),
        StoryEvent::ConflictDetected { .. } | StoryEvent::ConflictResolved { .. } => {}
    }
}
