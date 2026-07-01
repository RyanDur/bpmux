use super::{Stories, story_mut};
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::item::ProductItem;
use crate::product_management::story::{Story, StoryId, StoryKey, StoryState, StoryType};

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_story_created(
    w: &mut Stories,
    id: StoryId,
    meta: &EventMeta,
    title: &str,
    body: &str,
    initial_state: StoryState,
    story_type: StoryType,
) {
    let story = Story {
        id,
        title: title.to_string(),
        state: initial_state,
        filed_at: meta.ts,
        filed_by: meta.actor.clone(),
        body: Some(body.to_string()),
        free_comments: Some(Vec::new()),
        gate_comments: Some(Vec::new()),
        is_closed: false,
        raw_labels: Vec::new(),
        body_etag: None,
        story_type,
        acceptor: None,
        parent_epic: None,
        iteration_membership: None,
        tags: Vec::new(),
        ready_order: None,
    };
    w.insert(StoryKey::Saved(id), ProductItem::from_story(&story));
}

pub(super) fn apply_state_transitioned(w: &mut Stories, id: StoryId, new_state: StoryState) {
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(story))
        | Some(ProductItem::Iteration(story))
        | Some(ProductItem::Epic(story)) => story.state = new_state,
        None => {}
    }
}

pub(super) fn apply_body_fetched(w: &mut Stories, id: StoryId, body: &str, etag: &Option<String>) {
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(story))
        | Some(ProductItem::Iteration(story))
        | Some(ProductItem::Epic(story)) => {
            story.body = Some(body.to_string());
            story.body_etag = etag.clone();
        }
        None => {}
    }
}

pub(super) fn apply_title_updated(w: &mut Stories, id: StoryId, title: &str) {
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(story))
        | Some(ProductItem::Iteration(story))
        | Some(ProductItem::Epic(story)) => story.title = title.to_string(),
        None => {}
    }
}

pub(super) fn apply_body_updated(w: &mut Stories, id: StoryId, body: &str) {
    match w.get_mut(&StoryKey::Saved(id)) {
        Some(ProductItem::Story(story))
        | Some(ProductItem::Iteration(story))
        | Some(ProductItem::Epic(story)) => story.body = Some(body.to_string()),
        None => {}
    }
}

pub(super) fn apply_story_closed(w: &mut Stories, id: StoryId) {
    if let Some(story) = story_mut(w, id) {
        story.is_closed = true;
    }
}

pub(super) fn apply_story_reopened(w: &mut Stories, id: StoryId) {
    if let Some(story) = story_mut(w, id) {
        story.is_closed = false;
    }
}

pub(super) fn apply_story_refreshed(w: &mut Stories, id: StoryId, story: &Story) {
    let key = StoryKey::Saved(id);
    match w.get_mut(&key) {
        Some(ProductItem::Story(existing))
        | Some(ProductItem::Iteration(existing))
        | Some(ProductItem::Epic(existing)) => {
            existing.title = story.title.clone();
            existing.state = story.state;
            existing.raw_labels = story.raw_labels.clone();
            existing.is_closed = story.is_closed;
            if story.body.is_some() {
                existing.body = story.body.clone();
                existing.body_etag = None;
            }
        }
        _ => {
            w.insert(key, ProductItem::from_story(story));
        }
    }
}
