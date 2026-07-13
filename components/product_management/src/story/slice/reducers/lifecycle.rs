use super::{Stories, story_mut};
use crate::epic::EpicId;
use crate::story::events::EventMeta;
use crate::story::{Acceptor, Story, StoryId, StoryKey, StoryKind, Tag, Work};

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_story_created(
    w: &mut Stories,
    id: StoryId,
    meta: &EventMeta,
    title: &str,
    body: &str,
    initial_state: Work,
    story_type: StoryKind,
) {
    let story = Story {
        id,
        title: title.to_string(),
        state: initial_state,
        filed_at: meta.ts,
        filed_by: meta.actor.clone(),
        body: Some(body.to_string()),
        free_comments: Some(Vec::new()),
        verdicts: Some(Vec::new()),
        body_etag: None,
        story_type,
        acceptor: None,
        parent_epic: None,
        tags: Vec::new(),
    };
    w.insert(StoryKey::Saved(id), story);
}

pub(super) fn apply_state_transitioned(w: &mut Stories, id: StoryId, new_state: Work) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(id)) {
        story.state = new_state;
    }
}

pub(super) fn apply_body_fetched(w: &mut Stories, id: StoryId, body: &str, etag: &Option<String>) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(id)) {
        story.body = Some(body.to_string());
        story.body_etag = etag.clone();
    }
}

pub(super) fn apply_title_updated(w: &mut Stories, id: StoryId, title: &str) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(id)) {
        story.title = title.to_string();
    }
}

pub(super) fn apply_body_updated(w: &mut Stories, id: StoryId, body: &str) {
    if let Some(story) = w.get_mut(&StoryKey::Saved(id)) {
        story.body = Some(body.to_string());
    }
}

pub(super) fn apply_acceptor_assigned(w: &mut Stories, id: StoryId, acceptor: Acceptor) {
    if let Some(story) = story_mut(w, id) {
        story.acceptor = Some(acceptor);
    }
}

pub(super) fn apply_parent_epic_set(w: &mut Stories, id: StoryId, epic_id: EpicId) {
    if let Some(story) = story_mut(w, id) {
        story.parent_epic = Some(epic_id);
    }
}

pub(super) fn apply_parent_epic_unset(w: &mut Stories, id: StoryId) {
    if let Some(story) = story_mut(w, id) {
        story.parent_epic = None;
    }
}

pub(super) fn apply_tag_set(w: &mut Stories, id: StoryId, tag: Tag) {
    if let Some(story) = story_mut(w, id)
        && !story.tags.contains(&tag)
    {
        story.tags.push(tag);
    }
}

pub(super) fn apply_tag_unset(w: &mut Stories, id: StoryId, tag: Tag) {
    if let Some(story) = story_mut(w, id) {
        story.tags.retain(|t| *t != tag);
    }
}

pub(super) fn apply_story_refreshed(w: &mut Stories, id: StoryId, story: &Story) {
    let key = StoryKey::Saved(id);
    match w.get_mut(&key) {
        Some(existing) => {
            existing.title = story.title.clone();
            existing.state = story.state.clone();
            if story.body.is_some() {
                existing.body = story.body.clone();
                existing.body_etag = None;
            }
        }
        None => {
            w.insert(key, story.clone());
        }
    }
}
