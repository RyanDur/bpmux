use std::collections::HashMap;

use super::action::StoryAction;
use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::product_management::story::{Story, StoryId, StoryKey};

mod comments;
mod labels;
mod lifecycle;

use comments::{
    apply_free_comment_appended, apply_free_comments_fetched, apply_verdict_cast,
    apply_verdicts_fetched,
};
use labels::apply_labels_refreshed;
use lifecycle::{
    apply_acceptor_assigned, apply_body_fetched, apply_body_updated, apply_parent_epic_set,
    apply_parent_epic_unset, apply_state_transitioned, apply_story_created, apply_story_refreshed,
    apply_tag_set, apply_tag_unset, apply_title_updated,
};

pub(crate) type Stories = HashMap<StoryKey, Story>;

pub(super) fn story_mut(w: &mut Stories, id: StoryId) -> Option<&mut Story> {
    w.get_mut(&StoryKey::Saved(id))
}

pub fn reduce(state: &Stories, action: &StoryAction) -> Stories {
    let mut stories = state.clone();
    match action {
        StoryAction::Event(event) => apply_event(&mut stories, event),
        StoryAction::Events(events) => {
            for event in events {
                apply_event(&mut stories, event);
            }
        }
        StoryAction::Seeded(incoming) => {
            for item in incoming {
                let key = item.key();
                let mut seeded = item.clone();
                if let Some(existing) = stories.get(&key) {
                    seeded.verdicts = seeded.verdicts.or_else(|| existing.verdicts.clone());
                    seeded.free_comments = seeded
                        .free_comments
                        .or_else(|| existing.free_comments.clone());
                    if seeded.body.is_none() {
                        seeded.body = existing.body.clone();
                        seeded.body_etag = existing.body_etag.clone();
                    }
                }
                stories.insert(key, seeded);
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
    match &event.kind {
        StoryEventKind::StoryCreated {
            title,
            body,
            initial_state,
            story_type,
        } => apply_story_created(
            w,
            id,
            &event.meta,
            title,
            body,
            initial_state.clone(),
            *story_type,
        ),
        StoryEventKind::StateTransitioned { new_state, .. } => {
            apply_state_transitioned(w, id, new_state.clone())
        }
        StoryEventKind::VerdictCast { verdict, payload } => {
            apply_verdict_cast(w, id, &event.meta, verdict.clone(), payload.clone())
        }
        StoryEventKind::FreeCommentAppended { body } => {
            apply_free_comment_appended(w, id, &event.meta, body)
        }
        StoryEventKind::BodyFetched { body, etag } => apply_body_fetched(w, id, body, etag),
        StoryEventKind::FreeCommentsFetched { comments } => {
            apply_free_comments_fetched(w, id, comments)
        }
        StoryEventKind::VerdictsFetched { verdicts } => apply_verdicts_fetched(w, id, verdicts),
        StoryEventKind::StoryClosed { .. } => {}
        StoryEventKind::StoryReopened => {}
        StoryEventKind::AcceptorAssigned { acceptor } => apply_acceptor_assigned(w, id, *acceptor),
        StoryEventKind::ParentEpicSet { epic_id } => apply_parent_epic_set(w, id, *epic_id),
        StoryEventKind::ParentEpicUnset => apply_parent_epic_unset(w, id),
        StoryEventKind::TagSet { tag } => apply_tag_set(w, id, *tag),
        StoryEventKind::TagUnset { tag } => apply_tag_unset(w, id, *tag),
        StoryEventKind::TagAdded { .. }
        | StoryEventKind::TagRemoved { .. }
        | StoryEventKind::CurrentIterationDesignated
        | StoryEventKind::CurrentIterationCleared => {}
        StoryEventKind::StoryRefreshed { story } => apply_story_refreshed(w, id, story),
        StoryEventKind::TitleUpdated { title } => apply_title_updated(w, id, title),
        StoryEventKind::BodyUpdated { body } => apply_body_updated(w, id, body),
        StoryEventKind::LabelsRefreshed { fields } => apply_labels_refreshed(w, id, fields),
    }
}
