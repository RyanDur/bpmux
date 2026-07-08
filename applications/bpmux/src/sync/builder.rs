use super::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::sync::SourceAdapter;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn new(adapter: Arc<A>) -> Self {
        Self {
            story_dispatch: std::sync::OnceLock::new(),
            board_dispatch: std::sync::OnceLock::new(),
            adapter,
            status: Arc::new(Mutex::new(SyncStatus::default())),
            codec: Arc::new(crate::integrations::github::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(1),
            action_tx: None,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            drain_guard: tokio::sync::Mutex::new(()),
            ci_state_store: None,
            race_window_barrier: None,
        }
    }

    pub fn with_next_mutation_id(self, next: u64) -> Self {
        self.next_mutation_id.store(next.max(1), Ordering::SeqCst);
        self
    }

    pub fn with_ci_state_store(mut self, store: crate::ci::CiStateStore) -> Self {
        self.ci_state_store = Some(store);
        self
    }

    pub fn with_race_window_barrier(mut self, barrier: Arc<RaceWindowBarrier>) -> Self {
        self.race_window_barrier = Some(barrier);
        self
    }

    pub fn with_codec(
        mut self,
        codec: Arc<dyn crate::product_management::label_codec::LabelCodec + Send + Sync>,
    ) -> Self {
        self.codec = codec;
        self
    }

    pub fn with_action_tx(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>,
    ) -> Self {
        self.action_tx = Some(tx);
        self
    }
}
