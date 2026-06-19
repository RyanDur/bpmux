use std::collections::HashMap;

use crate::board::domain::Story;
use crate::board::events::BoardEvent;

mod comments;
mod labels;
mod lifecycle;

use comments::{
    apply_free_comment_appended, apply_free_comments_fetched, apply_gate_comment_appended,
    apply_gate_comments_fetched,
};
use labels::{apply_labels_refreshed, apply_tag_added, apply_tag_removed};
use lifecycle::{
    apply_body_fetched, apply_current_iteration_cleared, apply_current_iteration_designated,
    apply_iteration_membership_set, apply_outcome_membership_set, apply_parent_epic_set,
    apply_parent_epic_unset, apply_state_transitioned, apply_story_closed, apply_story_created,
    apply_story_refreshed, apply_story_reopened,
};

pub(super) type Stories = HashMap<u64, Story>;

pub(super) fn apply_event(w: &mut Stories, event: &BoardEvent) {
    let number = event.story_number();
    match event {
        BoardEvent::StoryCreated {
            meta,
            number: n,
            title,
            body,
            initial_state,
            story_type,
        } => apply_story_created(w, *n, meta, title, body, *initial_state, *story_type),
        BoardEvent::StateTransitioned { new_state, .. } => {
            apply_state_transitioned(w, number, *new_state)
        }
        BoardEvent::TagAdded { label, .. } => apply_tag_added(w, number, label),
        BoardEvent::TagRemoved { label, .. } => apply_tag_removed(w, number, label),
        BoardEvent::GateCommentAppended {
            meta,
            gate,
            payload,
            ..
        } => apply_gate_comment_appended(w, number, meta, *gate, payload.clone()),
        BoardEvent::FreeCommentAppended { meta, body, .. } => {
            apply_free_comment_appended(w, number, meta, body)
        }
        BoardEvent::BodyFetched { body, etag, .. } => apply_body_fetched(w, number, body, etag),
        BoardEvent::FreeCommentsFetched { comments, .. } => {
            apply_free_comments_fetched(w, number, comments)
        }
        BoardEvent::GateCommentsFetched { comments, .. } => {
            apply_gate_comments_fetched(w, number, comments)
        }
        BoardEvent::StoryClosed { .. } => apply_story_closed(w, number),
        BoardEvent::StoryReopened { .. } => apply_story_reopened(w, number),
        BoardEvent::ParentEpicSet { epic_number, .. } => {
            apply_parent_epic_set(w, number, *epic_number)
        }
        BoardEvent::ParentEpicUnset { .. } => apply_parent_epic_unset(w, number),
        BoardEvent::IterationMembershipSet {
            iteration_number,
            position,
            ..
        } => apply_iteration_membership_set(w, number, *iteration_number, *position),
        BoardEvent::OutcomeMembershipSet {
            outcome_number,
            position,
            ..
        } => apply_outcome_membership_set(w, number, *outcome_number, *position),
        BoardEvent::CurrentIterationDesignated { .. } => {
            apply_current_iteration_designated(w, number)
        }
        BoardEvent::CurrentIterationCleared { .. } => apply_current_iteration_cleared(w, number),
        BoardEvent::StoryRefreshed { story, .. } => apply_story_refreshed(w, number, story),
        BoardEvent::LabelsRefreshed { labels, .. } => apply_labels_refreshed(w, number, labels),
        BoardEvent::ConflictDetected { .. } | BoardEvent::ConflictResolved { .. } => {}
    }
}
