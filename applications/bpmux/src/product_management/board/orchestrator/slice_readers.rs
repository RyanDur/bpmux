use std::sync::Arc;

use super::{EpicReader, IterationReader, OutcomeReader, SyncOrchestrator};
use crate::product_management::board::sync::SourceAdapter;
use crate::product_management::epic::Epic;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub(super) fn set_outcome_reader(&self, reader: Arc<dyn OutcomeReader>) {
        let _ = self.outcome_reader.set(reader);
    }

    pub(super) fn read_outcomes(&self) -> Vec<Outcome> {
        self.outcome_reader
            .get()
            .map(|reader| reader.outcomes())
            .unwrap_or_default()
    }

    pub(super) fn set_iteration_reader(&self, reader: Arc<dyn IterationReader>) {
        let _ = self.iteration_reader.set(reader);
    }

    pub(super) fn read_iterations(&self) -> Vec<Iteration> {
        self.iteration_reader
            .get()
            .map(|reader| reader.iterations())
            .unwrap_or_default()
    }

    pub(super) fn set_epic_reader(&self, reader: Arc<dyn EpicReader>) {
        let _ = self.epic_reader.set(reader);
    }

    pub(super) fn read_epics(&self) -> Vec<Epic> {
        self.epic_reader
            .get()
            .map(|reader| reader.epics())
            .unwrap_or_default()
    }
}
