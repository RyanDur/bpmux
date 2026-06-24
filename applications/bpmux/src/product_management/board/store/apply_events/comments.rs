use super::Stories;
use crate::product_management::board::events::EventMeta;
use crate::product_management::story::{FreeComment, Gate, GateComment};

pub(super) fn apply_gate_comment_appended(
    w: &mut Stories,
    number: u64,
    meta: &EventMeta,
    gate: Gate,
    payload: Option<String>,
) {
    if let Some(story) = w.get_mut(&number) {
        let entry = GateComment::from_event(gate, meta, payload);
        match &mut story.gate_comments {
            Some(list) => list.push(entry),
            None => story.gate_comments = Some(vec![entry]),
        }
    }
}

pub(super) fn apply_free_comment_appended(
    w: &mut Stories,
    number: u64,
    meta: &EventMeta,
    body: &str,
) {
    if let Some(story) = w.get_mut(&number) {
        let entry = FreeComment::from_event(body.to_string(), meta);
        match &mut story.free_comments {
            Some(list) => list.push(entry),
            None => story.free_comments = Some(vec![entry]),
        }
    }
}

pub(super) fn apply_free_comments_fetched(w: &mut Stories, number: u64, comments: &[FreeComment]) {
    if let Some(story) = w.get_mut(&number) {
        story.free_comments = Some(comments.to_vec());
    }
}

pub(super) fn apply_gate_comments_fetched(w: &mut Stories, number: u64, comments: &[GateComment]) {
    if let Some(story) = w.get_mut(&number) {
        story.gate_comments = Some(comments.to_vec());
    }
}
