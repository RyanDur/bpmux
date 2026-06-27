use std::collections::HashMap;

use super::{BoardAction, BoardState};
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::item::BoardItem;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Story, StoryKey, StoryType};

mod comments;
mod labels;
mod lifecycle;

use comments::{
    apply_free_comment_appended, apply_free_comments_fetched, apply_gate_comment_appended,
    apply_gate_comments_fetched,
};
use labels::{apply_labels_refreshed, apply_raw_labels};
use lifecycle::{
    apply_body_fetched, apply_body_updated, apply_state_transitioned, apply_story_closed,
    apply_story_created, apply_story_refreshed, apply_story_reopened, apply_title_updated,
};

pub(super) type Stories = HashMap<StoryKey, BoardItem>;

pub(super) fn item_from_story(story: &Story) -> BoardItem {
    match story.story_type() {
        StoryType::Outcome => BoardItem::Outcome(Outcome::from_story(story)),
        StoryType::Story(_) | StoryType::Epic | StoryType::Iteration => {
            BoardItem::Story(story.clone())
        }
    }
}

pub(super) fn story_mut(w: &mut Stories, number: u64) -> Option<&mut Story> {
    w.get_mut(&StoryKey::Saved(number))
        .and_then(BoardItem::as_story_mut)
}

pub fn reduce(state: &BoardState, action: &BoardAction) -> BoardState {
    let mut stories = state.stories.clone();
    match action {
        BoardAction::Event(event) => apply_event(&mut stories, event),
        BoardAction::RawLabelsAssigned { number, fields } => {
            apply_raw_labels(&mut stories, *number, fields)
        }
        BoardAction::Seeded(incoming) => {
            for story in incoming {
                stories.insert(StoryKey::Saved(story.number), item_from_story(story));
            }
        }
        BoardAction::Removed(number) => {
            stories.remove(&StoryKey::Saved(*number));
        }
        BoardAction::Cleared => stories.clear(),
    }
    BoardState { stories }
}

fn apply_event(w: &mut Stories, event: &BoardEvent) {
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
        BoardEvent::TagAdded { .. }
        | BoardEvent::TagRemoved { .. }
        | BoardEvent::ParentEpicSet { .. }
        | BoardEvent::ParentEpicUnset { .. }
        | BoardEvent::IterationMembershipSet { .. }
        | BoardEvent::IterationMembershipCleared { .. }
        | BoardEvent::OutcomeMembershipSet { .. }
        | BoardEvent::OutcomeMembershipCleared { .. }
        | BoardEvent::CurrentIterationDesignated { .. }
        | BoardEvent::CurrentIterationCleared { .. } => {}
        BoardEvent::StoryRefreshed { story, .. } => apply_story_refreshed(w, number, story),
        BoardEvent::TitleUpdated { title, .. } => apply_title_updated(w, number, title),
        BoardEvent::BodyUpdated { body, .. } => apply_body_updated(w, number, body),
        BoardEvent::LabelsRefreshed { fields, .. } => apply_labels_refreshed(w, number, fields),
        BoardEvent::ConflictDetected { .. } | BoardEvent::ConflictResolved { .. } => {}
    }
}
