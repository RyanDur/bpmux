#[cfg(test)]
pub(crate) struct SeededStore {
    store: std::sync::Arc<kernel::Store<store::AppState, store::AppAction>>,
}

#[cfg(test)]
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

    pub(crate) fn new_wired_outbox(
        sink: crate::sync::OutboxSink,
        initial: Vec<source_sync::QueuedWrite>,
    ) -> Self {
        let store = kernel::Store::new(
            store::AppState::default().with_outbox(initial),
            Box::new(store::app_reduce),
        );
        store.add_listener(std::sync::Arc::new(crate::sync::OutboxPersister::new(sink)));
        Self { store }
    }

    pub(crate) fn pending_writes(&self) -> Vec<source_sync::QueuedWrite> {
        self.store
            .select(|state| state.sync.queue.iter().cloned().collect())
    }

    pub(crate) fn select(&self) -> store::Select {
        store::Select::new(std::sync::Arc::downgrade(&self.store))
    }

    pub(crate) fn build_orch<A: crate::sync::SourceAdapter + 'static>(
        &self,
        adapter: std::sync::Arc<A>,
        codec: std::sync::Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    ) -> std::sync::Arc<crate::sync::SyncOrchestrator> {
        std::sync::Arc::new(crate::sync::SyncOrchestrator::new(
            adapter,
            codec,
            self.select(),
            self.dispatcher(),
        ))
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

#[cfg(test)]
pub(crate) fn build_seeded_orch<A: crate::sync::SourceAdapter + 'static>(
    adapter: std::sync::Arc<A>,
    codec: std::sync::Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    outcomes: Vec<product_management::outcome::Outcome>,
    iterations: Vec<product_management::iteration::Iteration>,
    epics: Vec<product_management::epic::Epic>,
) -> (std::sync::Arc<crate::sync::SyncOrchestrator>, store::Select) {
    let handle = SeededStore::new(outcomes, iterations, epics);
    let orch = handle.build_orch(adapter, codec);
    let select = handle.select();
    std::mem::forget(handle);
    (orch, select)
}
