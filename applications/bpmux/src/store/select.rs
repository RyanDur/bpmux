use std::sync::Weak;

use super::AppAction;
use super::app::AppState;

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
        outcomes: Vec<product_management::outcome::Outcome>,
        iterations: Vec<product_management::iteration::Iteration>,
        epics: Vec<product_management::epic::Epic>,
    ) -> Self {
        let handle = Self {
            store: kernel::Store::new(AppState::default(), Box::new(super::app::app_reduce)),
        };
        if !outcomes.is_empty() {
            handle.dispatch(AppAction::Outcome(
                product_management::outcome::OutcomeAction::Upserted(outcomes),
            ));
        }
        if !iterations.is_empty() {
            handle.dispatch(AppAction::Iteration(
                product_management::iteration::IterationAction::Upserted(iterations),
            ));
        }
        if !epics.is_empty() {
            handle.dispatch(AppAction::Epic(
                product_management::epic::EpicAction::Upserted(epics),
            ));
        }
        handle
    }

    pub(crate) fn new_wired_outbox(
        sink: crate::sync::OutboxSink,
        initial: Vec<source_sync::QueuedWrite>,
    ) -> Self {
        let store = kernel::Store::new(
            AppState::default().with_outbox(initial),
            Box::new(super::app::app_reduce),
        );
        store.add_listener(std::sync::Arc::new(crate::sync::OutboxPersister::new(sink)));
        Self { store }
    }

    pub(crate) fn pending_writes(&self) -> Vec<source_sync::QueuedWrite> {
        self.store
            .select(|state| state.sync.queue.iter().cloned().collect())
    }

    pub(crate) fn select(&self) -> Select {
        Select::new(std::sync::Arc::downgrade(&self.store))
    }

    pub(crate) fn bind_refresh(&self, refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>) {
        refresh.bind_select(self.select());
        refresh.bind_dispatch(self.dispatcher());
    }

    pub(crate) fn dispatcher(&self) -> crate::store::Dispatch {
        let weak = std::sync::Arc::downgrade(&self.store);
        crate::store::Dispatch::new(std::sync::Arc::new(move |action| {
            if let Some(store) = weak.upgrade() {
                store.dispatch(action);
            }
        }))
    }

    pub(crate) fn dispatch(&self, action: AppAction) {
        self.store.dispatch(action);
    }
}

#[cfg(test)]
pub(crate) fn bind_seeded_refresh(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    outcomes: Vec<product_management::outcome::Outcome>,
    iterations: Vec<product_management::iteration::Iteration>,
    epics: Vec<product_management::epic::Epic>,
) -> Select {
    let handle = SeededStore::new(outcomes, iterations, epics);
    handle.bind_refresh(refresh);
    let select = handle.select();
    std::mem::forget(handle);
    select
}
