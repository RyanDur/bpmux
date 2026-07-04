use super::*;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

use crate::product_management::board::sync::SourceAdapter;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn new(adapter: Arc<A>) -> Self {
        Self {
            story_dispatch: std::sync::OnceLock::new(),
            adapter,
            status: Arc::new(Mutex::new(SyncStatus::default())),
            pending: Arc::new(Mutex::new(VecDeque::new())),
            outbox_path: None,
            storage: None,
            codec: Arc::new(crate::integrations::github::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(1),
            action_tx: None,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
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

    pub fn with_codec(
        mut self,
        codec: Arc<dyn crate::product_management::board::label_codec::LabelCodec + Send + Sync>,
    ) -> Self {
        self.codec = codec;
        self
    }

    pub fn with_outbox_path(adapter: Arc<A>, path: PathBuf) -> Result<Self, PersistError> {
        let pending = load_outbox_from_disk(&path)?;
        let next = pending.iter().map(|e| e.id.0).max().unwrap_or(0) + 1;
        Ok(Self {
            story_dispatch: std::sync::OnceLock::new(),
            adapter,
            status: Arc::new(Mutex::new(SyncStatus {
                pending_events: pending.len(),
                ..SyncStatus::default()
            })),
            pending: Arc::new(Mutex::new(pending)),
            outbox_path: Some(path),
            storage: None,
            codec: Arc::new(crate::integrations::github::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ci_state_store: None,
            race_window_barrier: None,
        })
    }

    pub fn with_storage(
        adapter: Arc<A>,
        storage: Arc<dyn crate::storage::Storage>,
    ) -> Result<Self, PersistError> {
        let pending = storage.load_outbox()?;
        let next = pending.iter().map(|e| e.id.0).max().unwrap_or(0) + 1;
        Ok(Self {
            story_dispatch: std::sync::OnceLock::new(),
            adapter,
            status: Arc::new(Mutex::new(SyncStatus {
                pending_events: pending.len(),
                ..SyncStatus::default()
            })),
            pending: Arc::new(Mutex::new(pending)),
            outbox_path: None,
            storage: Some(storage),
            codec: Arc::new(crate::integrations::github::gh_label_codec::GhLabelCodec),
            next_mutation_id: AtomicU64::new(next),
            action_tx: None,
            select: std::sync::OnceLock::new(),
            issue_locks: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            ci_state_store: None,
            race_window_barrier: None,
        })
    }

    pub fn with_action_tx(
        mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>,
    ) -> Self {
        self.action_tx = Some(tx);
        self
    }

    pub(super) fn persist_outbox(&self) {
        let snapshot: Vec<QueuedEvent> = self.with_pending(|p| p.iter().cloned().collect());
        if let Some(storage) = self.storage.as_ref() {
            if let Err(e) = storage.save_outbox(&snapshot) {
                crate::diag::log_diag(&format!(
                    "orchestrator: outbox persist via storage failed: {e}"
                ));
            }
            return;
        }
        let Some(path) = self.outbox_path.as_ref() else {
            return;
        };
        if let Err(e) = persist_outbox_to_disk(path, &snapshot) {
            crate::diag::log_diag(&format!("orchestrator: outbox persist failed: {e}"));
        }
    }
}
