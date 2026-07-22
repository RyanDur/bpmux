use std::collections::HashSet;

use super::SyncOrchestrator;
use source_sync::{MutationId, QueuedWrite};

use product_management::iteration::{self, Iteration};
use product_management::story::{self, Story, StoryId};

impl SyncOrchestrator {
    pub(super) fn read_pending_count(&self) -> usize {
        self.select
            .select(|state| store::sync::pending_count(&state.sync))
    }

    pub(super) fn read_health_snapshot(&self) -> store::sync_health::SyncHealthState {
        self.select
            .select(|state| store::sync_health::snapshot(&state.sync_health))
    }

    pub(super) fn read_dropped_count(&self) -> usize {
        self.select
            .select(|state| store::sync_health::dropped_count(&state.sync_health))
    }

    pub(super) fn read_next_unpushed(&self, skip: &HashSet<MutationId>) -> Option<QueuedWrite> {
        self.select
            .select(|state| store::sync::next_unpushed(&state.sync, skip))
    }

    pub(super) fn read_queued_for_story(&self, story_id: StoryId) -> usize {
        self.select
            .select(|state| store::sync::queued_for_story(&state.sync, story_id))
    }

    pub(super) fn read_queued_for_story_excluding(
        &self,
        story_id: StoryId,
        skip: &HashSet<MutationId>,
    ) -> usize {
        self.select
            .select(|state| store::sync::queued_for_story_excluding(&state.sync, story_id, skip))
    }

    pub(super) fn read_queued_story_ids(&self) -> Vec<StoryId> {
        self.select
            .select(|state| store::sync::queued_story_ids(&state.sync))
    }

    pub(crate) fn read_story(&self, id: StoryId) -> Option<Story> {
        self.select
            .select(|state| story::slice::get(&state.stories, id))
    }

    pub(super) fn read_iterations(&self) -> Vec<Iteration> {
        self.select
            .select(|state| iteration::all(&state.iterations))
    }
}
