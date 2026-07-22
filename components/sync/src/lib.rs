use std::sync::Arc;

use chrono::Utc;

use product_management::story::{CastVerdict, FreeComment};
pub use source_sync::cache_fill_meta;
pub use source_sync::{
    ConditionalBody, Ingested, PushContext, SourceAdapter, SyncError, classify_close_error,
    is_permanent_push_failure,
};
use source_sync::{SyncStatus, is_stale, is_startup_fail, staleness_threshold_secs};

mod backfill;
mod dispatch;
mod drain;
mod fetch;
mod ingest;
mod outbox_persister;
mod race_window;
mod slice_readers;
mod types;
mod write_through;

pub use backfill::backfill_shared_history;
pub use drain::GithubDrain;
pub use outbox_persister::{OutboxPersister, OutboxSink};
pub use race_window::*;
pub use types::*;

pub struct SyncOrchestrator {
    pub(crate) dispatch: store::Dispatch,
    pub(crate) adapter: Arc<dyn SourceAdapter>,
    pub(crate) codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    pub(crate) select: store::Select,
    pub(crate) issue_locks:
        tokio::sync::Mutex<std::collections::HashMap<u64, Arc<tokio::sync::Mutex<()>>>>,
    pub(crate) drain_guard: tokio::sync::Mutex<()>,
}

impl SyncOrchestrator {
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
mod outbox_persister_tests;
