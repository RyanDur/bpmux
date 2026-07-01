use std::collections::HashMap;

use super::action::StoryAction;
use crate::product_management::board::events::BoardEvent;
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
    apply_acceptor_assigned, apply_body_fetched, apply_body_updated, apply_parent_epic_set,
    apply_parent_epic_unset, apply_state_transitioned, apply_story_closed, apply_story_created,
    apply_story_refreshed, apply_story_reopened, apply_title_updated,
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

fn apply_event(w: &mut Stories, event: &BoardEvent) {
    let id = event.story_id();
    match event {
        BoardEvent::StoryCreated {
            meta,
            id: n,
            title,
            body,
            initial_state,
            story_type,
        } => apply_story_created(w, *n, meta, title, body, *initial_state, *story_type),
        BoardEvent::StateTransitioned { new_state, .. } => {
            apply_state_transitioned(w, id, *new_state)
        }
        BoardEvent::GateCommentAppended {
            meta,
            gate,
            payload,
            ..
        } => apply_gate_comment_appended(w, id, meta, *gate, payload.clone()),
        BoardEvent::FreeCommentAppended { meta, body, .. } => {
            apply_free_comment_appended(w, id, meta, body)
        }
        BoardEvent::BodyFetched { body, etag, .. } => apply_body_fetched(w, id, body, etag),
        BoardEvent::FreeCommentsFetched { comments, .. } => {
            apply_free_comments_fetched(w, id, comments)
        }
        BoardEvent::GateCommentsFetched { comments, .. } => {
            apply_gate_comments_fetched(w, id, comments)
        }
        BoardEvent::StoryClosed { .. } => apply_story_closed(w, id),
        BoardEvent::StoryReopened { .. } => apply_story_reopened(w, id),
        BoardEvent::AcceptorAssigned { acceptor, .. } => apply_acceptor_assigned(w, id, *acceptor),
        BoardEvent::ParentEpicSet { epic_id, .. } => apply_parent_epic_set(w, id, *epic_id),
        BoardEvent::ParentEpicUnset { .. } => apply_parent_epic_unset(w, id),
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::IterationMembershipSet { .. }
        | BoardEvent::IterationMembershipCleared { .. }
        | BoardEvent::CurrentIterationDesignated { .. }
        | BoardEvent::CurrentIterationCleared { .. } => {}
        BoardEvent::StoryRefreshed { story, .. } => apply_story_refreshed(w, id, story),
        BoardEvent::TitleUpdated { title, .. } => apply_title_updated(w, id, title),
        BoardEvent::BodyUpdated { body, .. } => apply_body_updated(w, id, body),
        BoardEvent::LabelsRefreshed { fields, .. } => apply_labels_refreshed(w, id, fields),
        BoardEvent::ConflictDetected { .. } | BoardEvent::ConflictResolved { .. } => {}
    }
}
