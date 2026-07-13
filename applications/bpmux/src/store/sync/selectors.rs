use std::collections::HashSet;

use crate::sync::{MutationId, QueuedEvent, QueuedWrite};
use product_management::story::StoryId;

use super::SyncState;

pub fn pending_count(state: &SyncState) -> usize {
    state.queue.len()
}

pub fn queued_for_story(state: &SyncState, story_id: StoryId) -> usize {
    state
        .queue
        .iter()
        .filter(|write| write.event.story_id().value() == story_id.value())
        .count()
}

pub fn queued_for_story_excluding(
    state: &SyncState,
    story_id: StoryId,
    skip: &HashSet<MutationId>,
) -> usize {
    state
        .queue
        .iter()
        .filter(|write| {
            write.event.story_id().value() == story_id.value() && !skip.contains(&write.id)
        })
        .count()
}

pub fn next_unpushed(state: &SyncState, skip: &HashSet<MutationId>) -> Option<QueuedWrite> {
    state
        .queue
        .iter()
        .find(|write| !skip.contains(&write.id))
        .cloned()
}

pub fn queued_story_ids(state: &SyncState) -> Vec<StoryId> {
    state
        .queue
        .iter()
        .map(|write| write.event.story_id())
        .collect()
}

pub fn queue_snapshot(state: &SyncState) -> Vec<QueuedEvent> {
    state
        .queue
        .iter()
        .map(|write| QueuedEvent::new_with_id(write.id, write.event.clone()))
        .collect()
}
