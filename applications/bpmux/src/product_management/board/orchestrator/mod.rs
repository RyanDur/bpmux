use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

use chrono::Utc;

use super::events::BoardEvent;
use super::store::{BoardStore, PersistError};
use super::sync::{SourceAdapter, SyncError};
use crate::product_management::story::{FreeComment, GateComment, Story};

mod coalesce;
mod conflict;
mod drain;
mod driver;
mod driver_ops;
mod fetch;
mod ingest;
mod outbox;
mod race_window;
mod status;
mod types;

use coalesce::{coalesce_iteration_label_events, stale_state_labels_for};
pub use conflict::{ConflictKind, ConflictResolveOrigin, classify_conflict};
pub use driver::*;
pub use outbox::{default_outbox_path, load_outbox_from_disk};
pub use race_window::*;
pub use status::{
    DroppedEvent, SyncStatus, SyncTransition, acknowledge_dropped, is_stale, is_startup_fail,
    staleness_threshold_secs, sync_transition,
};
use status::{
    decrement_pending, record_conflicts_pending, record_dropped, record_error, record_push_success,
    set_pending_events,
};
pub use types::*;

pub struct SyncOrchestrator<A: SourceAdapter + 'static> {
    pub(super) store: Arc<BoardStore>,
    pub(super) adapter: Arc<A>,
    pub(super) status: Arc<Mutex<SyncStatus>>,
    pub(super) pending: Arc<Mutex<VecDeque<QueuedEvent>>>,
    pub(super) outbox_path: Option<PathBuf>,
    pub(super) storage: Option<Arc<dyn crate::storage::Storage>>,
    pub(super) codec:
        Arc<dyn crate::product_management::board::label_codec::LabelCodec + Send + Sync>,
    pub(super) next_mutation_id: AtomicU64,
    pub(super) action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::StoreAction>>,
    pub(super) issue_locks:
        tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
    pub(super) ci_state_store: Option<crate::transport::ci_state_store::CiStateStore>,
    pub(super) race_window_barrier: Option<Arc<RaceWindowBarrier>>,
}

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub fn status_handle(&self) -> Arc<Mutex<SyncStatus>> {
        Arc::clone(&self.status)
    }

    pub(super) fn with_pending<R>(&self, f: impl FnOnce(&VecDeque<QueuedEvent>) -> R) -> R {
        let p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&p)
    }

    pub(super) fn with_pending_mut<R>(&self, f: impl FnOnce(&mut VecDeque<QueuedEvent>) -> R) -> R {
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut p)
    }

    #[cfg(test)]
    pub fn snapshot_pending_for_test(&self) -> VecDeque<QueuedEvent> {
        self.with_pending(|p| p.clone())
    }

    pub fn store(&self) -> Arc<BoardStore> {
        Arc::clone(&self.store)
    }

    pub fn apply_server_rollback(&self, story: Story) {
        self.store.seed_all(vec![story]);
    }

    pub fn pending_issue_for_mutation(&self, id: MutationId) -> Option<u64> {
        self.with_pending(|p| {
            p.iter()
                .find(|q| q.id == id)
                .map(|q| q.event.story_id().value())
        })
    }

    pub async fn issue_lock(&self, issue: u64) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.issue_locks.lock().await;
        Arc::clone(
            locks
                .entry(issue)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }

    pub fn snapshot_status(&self) -> SyncStatus {
        let snapshot = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        SyncStatus {
            stale: is_stale(
                snapshot.last_ingest_at,
                staleness_threshold_secs(),
                Utc::now(),
            ),
            startup_fail: is_startup_fail(snapshot.last_ingest_at, snapshot.last_error.as_deref()),
            ..snapshot
        }
    }

    pub async fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> Result<u64, SyncError> {
        match self.adapter.create_story(title, body, initial_labels).await {
            Ok(number) => {
                record_push_success(&self.status);
                Ok(number)
            }
            Err(e) => {
                record_error(&self.status, &format!("create_story: {e}"));
                Err(e)
            }
        }
    }

    pub async fn reconcile_dual_state_labels(&self) -> Result<usize, SyncError> {
        let stories = self.store.all_stories();
        let mut repairs = 0usize;
        for story in stories {
            let labels = match self.adapter.fetch_story_labels(story.id.value()).await {
                Ok(l) => l,
                Err(_) => continue,
            };
            let stale = stale_state_labels_for(&story, &labels);
            for stale_label in stale {
                let meta = super::events::EventMeta {
                    actor: "bpmux:reconcile".to_string(),
                    ts: Utc::now(),
                    peer_id: super::peer::PeerId::local(),
                };
                self.mutate(BoardEvent::TagRemoved {
                    meta,
                    id: story.id,
                    label: stale_label,
                })?;
                repairs += 1;
            }
        }
        if repairs > 0 {
            self.drain_once().await?;
        }
        Ok(repairs)
    }
}

use outbox::persist_outbox_to_disk;

mod builder;
