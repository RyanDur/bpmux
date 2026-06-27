use super::{Stories, item_from_story, story_mut};
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::item::BoardItem;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Story, StoryKey, StoryState, StoryType};

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_story_created(
    w: &mut Stories,
    number: u64,
    meta: &EventMeta,
    title: &str,
    body: &str,
    initial_state: StoryState,
    story_type: StoryType,
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
        story_type,
        acceptor: None,
        parent_epic: None,
        iteration_membership: None,
        tags: Vec::new(),
        ready_order: None,
    };
    w.insert(StoryKey::Saved(number), item_from_story(&story));
}

pub(super) fn apply_state_transitioned(w: &mut Stories, number: u64, new_state: StoryState) {
    match w.get_mut(&StoryKey::Saved(number)) {
        Some(BoardItem::Story(story)) => story.state = new_state,
        Some(BoardItem::Outcome(outcome)) if new_state == StoryState::Retired => {
            *outcome = outcome.retired();
        }
        _ => {}
    }
}

pub(super) fn apply_body_fetched(w: &mut Stories, number: u64, body: &str, etag: &Option<String>) {
    match w.get_mut(&StoryKey::Saved(number)) {
        Some(BoardItem::Story(story)) => {
            story.body = Some(body.to_string());
            story.body_etag = etag.clone();
        }
        Some(BoardItem::Outcome(outcome)) => *outcome = outcome.with_body(body),
        None => {}
    }
}

pub(super) fn apply_title_updated(w: &mut Stories, number: u64, title: &str) {
    match w.get_mut(&StoryKey::Saved(number)) {
        Some(BoardItem::Story(story)) => story.title = title.to_string(),
        Some(BoardItem::Outcome(outcome)) => *outcome = outcome.with_title(title),
        None => {}
    }
}

pub(super) fn apply_body_updated(w: &mut Stories, number: u64, body: &str) {
    match w.get_mut(&StoryKey::Saved(number)) {
        Some(BoardItem::Story(story)) => story.body = Some(body.to_string()),
        Some(BoardItem::Outcome(outcome)) => *outcome = outcome.with_body(body),
        None => {}
    }
}

pub(super) fn apply_story_closed(w: &mut Stories, number: u64) {
    if let Some(story) = story_mut(w, number) {
        story.is_closed = true;
    }
}

pub(super) fn apply_story_reopened(w: &mut Stories, number: u64) {
    if let Some(story) = story_mut(w, number) {
        story.is_closed = false;
    }
}

pub(super) fn apply_story_refreshed(w: &mut Stories, number: u64, story: &Story) {
    let key = StoryKey::Saved(number);
    if story.story_type() == StoryType::Outcome {
        let existing_body = w
            .get(&key)
            .and_then(BoardItem::as_outcome)
            .and_then(|outcome| outcome.content.body.clone());
        let base = Outcome::from_story(story);
        let rebuilt = match (&base.content.body, existing_body) {
            (None, Some(body)) => base.with_body(&body),
            _ => base,
        };
        w.insert(key, BoardItem::Outcome(rebuilt));
        return;
    }
    match w.get_mut(&key) {
        Some(BoardItem::Story(existing)) => {
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
            w.insert(key, BoardItem::Story(story.clone()));
        }
    }
}
