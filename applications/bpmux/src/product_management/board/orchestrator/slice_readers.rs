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
        match self.select.get() {
            Some(s) => s.select(|state| state.stories.get(&StoryKey::Saved(id)).cloned()),
            None => self.store.story(id),
        }
    }

    pub(super) fn read_all_stories(&self) -> Vec<Story> {
        match self.select.get() {
            Some(s) => s.select(|state| state.stories.values().cloned().collect()),
            None => self.store.all_stories(),
        }
    }

    pub(super) fn read_store_is_empty(&self) -> bool {
        match self.select.get() {
            Some(s) => s.select(|state| state.stories.is_empty()),
            None => self.store.is_empty(),
        }
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
