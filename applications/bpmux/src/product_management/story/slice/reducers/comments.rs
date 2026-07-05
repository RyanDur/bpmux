use super::{Stories, story_mut};
use crate::product_management::story::events::EventMeta;
use crate::product_management::story::{CastVerdict, FreeComment, StoryId, Verdict};

pub(super) fn apply_verdict_cast(
    w: &mut Stories,
    id: StoryId,
    meta: &EventMeta,
    verdict: Verdict,
    payload: Option<String>,
) {
    if let Some(story) = story_mut(w, id) {
        let entry = CastVerdict::from_event(verdict, meta, payload);
        match &mut story.verdicts {
            Some(list) => list.push(entry),
            None => story.verdicts = Some(vec![entry]),
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

pub(super) fn apply_verdicts_fetched(w: &mut Stories, id: StoryId, verdicts: &[CastVerdict]) {
    if let Some(story) = story_mut(w, id) {
        story.verdicts = Some(verdicts.to_vec());
    }
}
