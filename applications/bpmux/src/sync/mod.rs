use std::sync::Arc;

use chrono::Utc;

use product_management::story::{CastVerdict, FreeComment};
pub(crate) use source_sync::cache_fill_meta;
pub use source_sync::{
    ConditionalBody, Ingested, PushContext, SourceAdapter, SyncError, classify_close_error,
    is_permanent_push_failure,
};
use source_sync::{SyncStatus, is_stale, is_startup_fail, staleness_threshold_secs};

mod backfill;
mod dispatch;
mod drain;
mod driver;
mod driver_ops;
mod fetch;
mod ingest;
mod outbox_persister;
mod race_window;
mod slice_readers;
mod types;
mod write_through;

pub(crate) use backfill::backfill_shared_history;
pub(crate) use drain::GithubDrain;
pub use driver::*;
pub use outbox_persister::{OutboxPersister, OutboxSink};
pub use race_window::*;
pub use types::*;

pub struct SyncOrchestrator<A: SourceAdapter + 'static> {
    pub(super) dispatch: std::sync::OnceLock<crate::store::Dispatch>,
    pub(super) adapter: Arc<A>,
    pub(super) codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    pub(super) select: std::sync::OnceLock<crate::store::Select>,
    pub(super) issue_locks:
        tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
    pub(super) drain_guard: tokio::sync::Mutex<()>,
    pub(super) ci_state_store: Option<crate::ci::CiStateStore>,
    pub(super) race_window_barrier: Option<Arc<RaceWindowBarrier>>,
}

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn issue_lock(&self, issue: u64) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.issue_locks.lock().await;
        Arc::clone(
            locks
                .entry(issue)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }

    pub fn snapshot_status(&self) -> SyncStatus {
        let health = self.read_health_snapshot();
        let stale = is_stale(
            health.last_ingest_at,
            staleness_threshold_secs(),
            Utc::now(),
        );
        let startup_fail = is_startup_fail(health.last_ingest_at, health.last_error.as_deref());
        SyncStatus {
            last_ingest_at: health.last_ingest_at,
            last_push_at: health.last_push_at,
            pending_events: self.read_pending_count(),
            last_error: health.last_error,
            dropped_events: health.dropped_events,
            stale,
            startup_fail,
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
                self.record_push_success();
                Ok(number)
            }
            Err(e) => {
                self.record_error(&format!("create_story: {e}"));
                Err(e)
            }
        }
    }
}

mod builder;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod orchestrator_tests;
#[cfg(test)]
mod outbox_persister_tests;
