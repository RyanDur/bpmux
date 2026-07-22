pub struct SeededStore {
    store: std::sync::Arc<kernel::Store<store::AppState, store::AppAction>>,
}

impl SeededStore {
    pub fn new(
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

    pub fn new_wired_outbox(
        sink: sync::OutboxSink,
        initial: Vec<source_sync::QueuedWrite>,
    ) -> Self {
        let store = kernel::Store::new(
            store::AppState::default().with_outbox(initial),
            Box::new(store::app_reduce),
        );
        store.add_listener(std::sync::Arc::new(sync::OutboxPersister::new(sink)));
        Self { store }
    }

    pub fn pending_writes(&self) -> Vec<source_sync::QueuedWrite> {
        self.store
            .select(|state| state.sync.queue.iter().cloned().collect())
    }

    pub fn select(&self) -> store::Select {
        store::Select::new(std::sync::Arc::downgrade(&self.store))
    }

    pub fn build_orch<A: sync::SourceAdapter + 'static>(
        &self,
        adapter: std::sync::Arc<A>,
        codec: std::sync::Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    ) -> std::sync::Arc<sync::SyncOrchestrator> {
        std::sync::Arc::new(sync::SyncOrchestrator::new(
            adapter,
            codec,
            self.select(),
            self.dispatcher(),
        ))
    }

    pub fn dispatcher(&self) -> store::Dispatch {
        let weak = std::sync::Arc::downgrade(&self.store);
        store::Dispatch::new(std::sync::Arc::new(move |action| {
            if let Some(store) = weak.upgrade() {
                store.dispatch(action);
            }
        }))
    }

    pub fn dispatch(&self, action: store::AppAction) {
        self.store.dispatch(action);
    }
}

pub fn build_seeded_orch<A: sync::SourceAdapter + 'static>(
    adapter: std::sync::Arc<A>,
    codec: std::sync::Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    outcomes: Vec<product_management::outcome::Outcome>,
    iterations: Vec<product_management::iteration::Iteration>,
    epics: Vec<product_management::epic::Epic>,
) -> (std::sync::Arc<sync::SyncOrchestrator>, store::Select) {
    let handle = SeededStore::new(outcomes, iterations, epics);
    let orch = handle.build_orch(adapter, codec);
    let select = handle.select();
    std::mem::forget(handle);
    (orch, select)
}

#[allow(clippy::panic)]
pub async fn eventually(label: &str, mut condition: impl FnMut() -> bool) {
    for _ in 0..400 {
        if condition() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("eventually({label}): condition not satisfied within 2s");
}
