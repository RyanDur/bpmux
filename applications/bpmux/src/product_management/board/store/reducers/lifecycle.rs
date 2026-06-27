use super::Stories;
use crate::product_management::board::events::EventMeta;
use crate::product_management::story::{Story, StoryState};

pub(super) fn apply_story_created(
    w: &mut Stories,
    number: u64,
    meta: &EventMeta,
    title: &str,
    body: &str,
    initial_state: StoryState,
) {
    let story = Story {
        number,
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
    };
    w.insert(number, story);
}

pub(super) fn apply_state_transitioned(w: &mut Stories, number: u64, new_state: StoryState) {
    if let Some(story) = w.get_mut(&number) {
        story.state = new_state;
    }
}

pub(super) fn apply_body_fetched(w: &mut Stories, number: u64, body: &str, etag: &Option<String>) {
    if let Some(story) = w.get_mut(&number) {
        story.body = Some(body.to_string());
        story.body_etag = etag.clone();
    }
}

pub(super) fn apply_title_updated(w: &mut Stories, number: u64, title: &str) {
    if let Some(story) = w.get_mut(&number) {
        story.title = title.to_string();
    }
}

pub(super) fn apply_body_updated(w: &mut Stories, number: u64, body: &str) {
    if let Some(story) = w.get_mut(&number) {
        story.body = Some(body.to_string());
    }
}

pub(super) fn apply_story_closed(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.is_closed = true;
    }
}

pub(super) fn apply_story_reopened(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.is_closed = false;
    }
}

pub(super) fn apply_story_refreshed(w: &mut Stories, number: u64, story: &Story) {
    if let Some(existing) = w.get_mut(&number) {
        existing.title = story.title.clone();
        existing.state = story.state;
        existing.raw_labels = story.raw_labels.clone();
        existing.is_closed = story.is_closed;
        if story.body.is_some() {
            existing.body = story.body.clone();
            existing.body_etag = None;
        }
    } else {
        w.insert(number, story.clone());
    }
}
