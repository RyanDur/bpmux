use std::sync::Weak;

use super::AppAction;
use super::app::AppState;
use crate::kernel;

#[derive(Clone)]
pub struct Select {
    app: Weak<kernel::Store<AppState, AppAction>>,
}

impl Select {
    pub fn new(app: Weak<kernel::Store<AppState, AppAction>>) -> Self {
        Self { app }
    }

    pub fn select<T: Default>(&self, selector: impl FnOnce(&AppState) -> T) -> T {
        match self.app.upgrade() {
            Some(store) => store.select(selector),
            None => T::default(),
        }
    }
}

#[cfg(test)]
pub(crate) struct SeededStore {
    store: std::sync::Arc<kernel::Store<AppState, AppAction>>,
}

#[cfg(test)]
impl SeededStore {
    pub(crate) fn new(
        outcomes: Vec<crate::product_management::outcome::Outcome>,
        iterations: Vec<crate::product_management::iteration::Iteration>,
        epics: Vec<crate::product_management::epic::Epic>,
    ) -> Self {
        let handle = Self {
            store: kernel::Store::new(AppState::default(), Box::new(super::app::app_reduce)),
        };
        if !outcomes.is_empty() {
            handle.dispatch(AppAction::Outcome(
                crate::product_management::outcome::OutcomeAction::Upserted(outcomes),
            ));
        }
        if !iterations.is_empty() {
            handle.dispatch(AppAction::Iteration(
                crate::product_management::iteration::IterationAction::Upserted(iterations),
            ));
        }
        if !epics.is_empty() {
            handle.dispatch(AppAction::Epic(
                crate::product_management::epic::EpicAction::Upserted(epics),
            ));
        }
        handle
    }

    pub(crate) fn select(&self) -> Select {
        Select::new(std::sync::Arc::downgrade(&self.store))
    }

    pub(crate) fn bind_refresh(&self, refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>) {
        refresh.bind_select(self.select());
        let weak = std::sync::Arc::downgrade(&self.store);
        refresh.bind_story_dispatch(Box::new(move |action| {
            if let Some(store) = weak.upgrade() {
                store.dispatch(AppAction::Story(action));
            }
        }));
    }

    pub(crate) fn dispatch(&self, action: AppAction) {
        self.store.dispatch(action);
    }
}

#[cfg(test)]
pub(crate) fn bind_seeded_refresh(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    outcomes: Vec<crate::product_management::outcome::Outcome>,
    iterations: Vec<crate::product_management::iteration::Iteration>,
    epics: Vec<crate::product_management::epic::Epic>,
) {
    let handle = SeededStore::new(outcomes, iterations, epics);
    handle.bind_refresh(refresh);
    std::mem::forget(handle);
}
