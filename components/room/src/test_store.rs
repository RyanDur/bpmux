pub(crate) struct SeededStore {
    store: std::sync::Arc<kernel::Store<store::AppState, store::AppAction>>,
}

impl SeededStore {
    pub(crate) fn new(
        outcomes: Vec<product_management::outcome::Outcome>,
        iterations: Vec<product_management::iteration::Iteration>,
        epics: Vec<product_management::epic::Epic>,
    ) -> Self {
        let handle = Self {
            store: kernel::Store::new(store::AppState::default(), Box::new(store::app_reduce)),
        };
        if !outcomes.is_empty() {
            handle.dispatch(store::AppAction::Outcome(
                product_management::outcome::OutcomeAction::Upserted(outcomes),
            ));
        }
        if !iterations.is_empty() {
            handle.dispatch(store::AppAction::Iteration(
                product_management::iteration::IterationAction::Upserted(iterations),
            ));
        }
        if !epics.is_empty() {
            handle.dispatch(store::AppAction::Epic(
                product_management::epic::EpicAction::Upserted(epics),
            ));
        }
        handle
    }

    pub(crate) fn select(&self) -> store::Select {
        store::Select::new(std::sync::Arc::downgrade(&self.store))
    }

    pub(crate) fn dispatcher(&self) -> store::Dispatch {
        let weak = std::sync::Arc::downgrade(&self.store);
        store::Dispatch::new(std::sync::Arc::new(move |action| {
            if let Some(store) = weak.upgrade() {
                store.dispatch(action);
            }
        }))
    }

    pub(crate) fn dispatch(&self, action: store::AppAction) {
        self.store.dispatch(action);
    }
}
