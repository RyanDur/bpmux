use std::collections::HashSet;

use super::SyncOrchestrator;
use source_sync::{MutationId, QueuedWrite};

use crate::sync::SourceAdapter;
use product_management::epic::{self, Epic};
use product_management::iteration::{self, Iteration};
use product_management::outcome::{self, Outcome};
use product_management::story::{self, Stage, Story, StoryId};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub(super) fn set_select(&self, select: crate::store::Select) {
        let _ = self.select.set(select);
    }

    pub(super) fn read_pending_count(&self) -> usize {
        match self.select.get() {
            Some(s) => s.select(|state| crate::store::sync::pending_count(&state.sync)),
            None => 0,
        }
    }

    pub(super) fn read_health_snapshot(&self) -> crate::store::sync_health::SyncHealthState {
        match self.select.get() {
            Some(s) => s.select(|state| crate::store::sync_health::snapshot(&state.sync_health)),
            None => crate::store::sync_health::SyncHealthState::default(),
        }
    }

    pub(super) fn read_dropped_count(&self) -> usize {
        match self.select.get() {
            Some(s) => {
                s.select(|state| crate::store::sync_health::dropped_count(&state.sync_health))
            }
            None => 0,
        }
    }

    pub(super) fn read_next_unpushed(&self, skip: &HashSet<MutationId>) -> Option<QueuedWrite> {
        match self.select.get() {
            Some(s) => s.select(|state| crate::store::sync::next_unpushed(&state.sync, skip)),
            None => None,
        }
    }

    pub(super) fn read_queued_for_story(&self, story_id: StoryId) -> usize {
        match self.select.get() {
            Some(s) => {
                s.select(|state| crate::store::sync::queued_for_story(&state.sync, story_id))
            }
            None => 0,
        }
    }

    pub(super) fn read_queued_for_story_excluding(
        &self,
        story_id: StoryId,
        skip: &HashSet<MutationId>,
    ) -> usize {
        match self.select.get() {
            Some(s) => s.select(|state| {
                crate::store::sync::queued_for_story_excluding(&state.sync, story_id, skip)
            }),
            None => 0,
        }
    }

    pub(super) fn read_queued_story_ids(&self) -> Vec<StoryId> {
        match self.select.get() {
            Some(s) => s.select(|state| crate::store::sync::queued_story_ids(&state.sync)),
            None => Vec::new(),
        }
    }

    pub(super) fn read_story(&self, id: StoryId) -> Option<Story> {
        self.select
            .get()
            .and_then(|s| s.select(|state| story::slice::get(&state.stories, id)))
    }

    pub(super) fn read_all_stories(&self) -> Vec<Story> {
        self.select
            .get()
            .map(|s| s.select(|state| story::slice::all(&state.stories)))
            .unwrap_or_default()
    }

    pub(super) fn read_story_fine_stage(&self, story: &Story) -> Option<Stage> {
        self.select
            .get()
            .and_then(|s| s.select(|state| Some(board::fine_stage(story, &state.board))))
            .unwrap_or_else(|| board::fine_stage(story, &board::BoardPlacements::default()))
    }

    pub(super) fn read_story_held_stage(&self, story: &Story) -> Option<workflow::StageId> {
        self.select
            .get()
            .and_then(|s| s.select(|state| Some(board::held_stage(story, &state.board))))
            .unwrap_or_else(|| board::held_stage(story, &board::BoardPlacements::default()))
    }

    pub(super) fn read_store_is_empty(&self) -> bool {
        self.select
            .get()
            .map(|s| s.select(|state| story::slice::is_empty(&state.stories)))
            .unwrap_or(true)
    }

    pub(super) fn read_evidence_for_story(
        &self,
        story_id: StoryId,
    ) -> Vec<product_management::evidence::Evidence> {
        self.select
            .get()
            .map(|s| {
                s.select(|state| {
                    product_management::evidence::evidence_for_story(&state.evidence, story_id)
                })
            })
            .unwrap_or_default()
    }

    pub(super) fn read_outcomes(&self) -> Vec<Outcome> {
        self.select
            .get()
            .map(|s| s.select(|state| outcome::all(&state.outcomes)))
            .unwrap_or_default()
    }

    pub(super) fn read_iterations(&self) -> Vec<Iteration> {
        self.select
            .get()
            .map(|s| s.select(|state| iteration::all(&state.iterations)))
            .unwrap_or_default()
    }

    pub(super) fn read_epics(&self) -> Vec<Epic> {
        self.select
            .get()
            .map(|s| s.select(|state| epic::all(&state.epics)))
            .unwrap_or_default()
    }
}
