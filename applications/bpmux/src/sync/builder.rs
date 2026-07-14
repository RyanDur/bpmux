use super::*;
use std::sync::Arc;

use crate::sync::SourceAdapter;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn new(
        adapter: Arc<A>,
        codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    ) -> Self {
        Self {
            dispatch: std::sync::OnceLock::new(),
            adapter,
            codec,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            drain_guard: tokio::sync::Mutex::new(()),
            ci_state_store: None,
            race_window_barrier: None,
        }
    }

    pub fn with_ci_state_store(mut self, store: crate::ci::CiStateStore) -> Self {
        self.ci_state_store = Some(store);
        self
    }

    pub fn with_race_window_barrier(mut self, barrier: Arc<RaceWindowBarrier>) -> Self {
        self.race_window_barrier = Some(barrier);
        self
    }
}
