use super::SyncOrchestrator;
use crate::product_management::board::sync::SourceAdapter;
use crate::product_management::epic::{self, Epic};
use crate::product_management::iteration::{self, Iteration};
use crate::product_management::outcome::{self, Outcome};
use crate::product_management::story::{Story, StoryId, StoryKey};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub(super) fn set_select(&self, select: crate::store::Select) {
        let _ = self.select.set(select);
    }

    pub(super) fn read_story(&self, id: StoryId) -> Option<Story> {
        self.select
            .get()
            .and_then(|s| s.select(|state| state.stories.get(&StoryKey::Saved(id)).cloned()))
    }

    pub(super) fn read_all_stories(&self) -> Vec<Story> {
        self.select
            .get()
            .map(|s| s.select(|state| state.stories.values().cloned().collect()))
            .unwrap_or_default()
    }

    pub(super) fn read_store_is_empty(&self) -> bool {
        self.select
            .get()
            .map(|s| s.select(|state| state.stories.is_empty()))
            .unwrap_or(true)
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
