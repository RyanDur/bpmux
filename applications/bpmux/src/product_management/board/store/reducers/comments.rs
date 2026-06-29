use super::{Stories, story_mut};
use crate::product_management::board::events::EventMeta;
use crate::product_management::story::{FreeComment, Gate, GateComment, StoryId};

pub(super) fn apply_gate_comment_appended(
    w: &mut Stories,
    id: StoryId,
    meta: &EventMeta,
    gate: Gate,
    payload: Option<String>,
) {
    if let Some(story) = story_mut(w, id) {
        let entry = GateComment::from_event(gate, meta, payload);
        match &mut story.gate_comments {
            Some(list) => list.push(entry),
            None => story.gate_comments = Some(vec![entry]),
        }
    }
}

pub(super) fn apply_free_comment_appended(
    w: &mut Stories,
    id: StoryId,
    meta: &EventMeta,
    body: &str,
) {
    if let Some(story) = story_mut(w, id) {
        let entry = FreeComment::from_event(body.to_string(), meta);
        match &mut story.free_comments {
            Some(list) => list.push(entry),
            None => story.free_comments = Some(vec![entry]),
        }
    }
}

pub(super) fn apply_free_comments_fetched(w: &mut Stories, id: StoryId, comments: &[FreeComment]) {
    if let Some(story) = story_mut(w, id) {
        story.free_comments = Some(comments.to_vec());
    }
}

pub(super) fn apply_gate_comments_fetched(w: &mut Stories, id: StoryId, comments: &[GateComment]) {
    if let Some(story) = story_mut(w, id) {
        story.gate_comments = Some(comments.to_vec());
    }
}
