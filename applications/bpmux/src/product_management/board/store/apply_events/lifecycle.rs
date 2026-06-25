use super::Stories;
use crate::product_management::board::events::EventMeta;
use crate::product_management::story::{Story, StoryState, StoryType};

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
        story_type,
        tags: Vec::new(),
        filed_at: meta.ts,
        filed_by: meta.actor.clone(),
        body: Some(body.to_string()),
        free_comments: Some(Vec::new()),
        gate_comments: Some(Vec::new()),
        is_closed: false,
        parent_epic: None,
        acceptance_test_path: crate::bridge::parse_acceptance_test_path(body),
        raw_labels: Vec::new(),
        body_etag: None,
        iteration_membership: None,
        horizon: None,
        outcome_membership: None,
        is_current: false,
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
        story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
        story.body_etag = etag.clone();
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

pub(super) fn apply_parent_epic_set(w: &mut Stories, number: u64, epic_number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.parent_epic = Some(epic_number);
    }
}

pub(super) fn apply_parent_epic_unset(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.parent_epic = None;
    }
}

pub(super) fn apply_iteration_membership_set(
    w: &mut Stories,
    number: u64,
    iteration_number: u64,
    position: u32,
) {
    if let Some(story) = w.get_mut(&number) {
        story.iteration_membership = Some((iteration_number, position));
    }
}

pub(super) fn apply_iteration_membership_cleared(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.iteration_membership = None;
    }
}

pub(super) fn apply_outcome_membership_cleared(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.outcome_membership = None;
    }
}

pub(super) fn apply_outcome_membership_set(
    w: &mut Stories,
    number: u64,
    outcome_number: u64,
    position: u32,
) {
    if let Some(story) = w.get_mut(&number) {
        story.outcome_membership = Some((outcome_number, position));
    }
}

pub(super) fn apply_current_iteration_designated(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.is_current = true;
    }
}

pub(super) fn apply_current_iteration_cleared(w: &mut Stories, number: u64) {
    if let Some(story) = w.get_mut(&number) {
        story.is_current = false;
    }
}

pub(super) fn apply_story_refreshed(w: &mut Stories, number: u64, story: &Story) {
    if let Some(existing) = w.get_mut(&number) {
        existing.title = story.title.clone();
        existing.state = story.state;
        existing.story_type = story.story_type;
        existing.raw_labels = story.raw_labels.clone();
        existing.tags = story.tags.clone();
        existing.is_closed = story.is_closed;
        existing.parent_epic = story.parent_epic;
        existing.iteration_membership = story.iteration_membership;
        existing.outcome_membership = story.outcome_membership;
        existing.horizon = story.horizon;
        existing.is_current = story.is_current;
        if story.body.is_some() {
            existing.body = story.body.clone();
            existing.acceptance_test_path = story.acceptance_test_path.clone();
            existing.body_etag = None;
        }
    } else {
        w.insert(number, story.clone());
    }
}
