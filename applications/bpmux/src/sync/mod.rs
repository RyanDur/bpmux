use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Utc;

pub use persist::PersistError;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{CastVerdict, FreeComment};

mod backfill;
mod coalesce;
mod dispatch;
mod drain;
mod driver;
mod driver_ops;
mod fetch;
mod ingest;
mod outbox;
mod port;
mod race_window;
mod slice_readers;
mod status;
mod types;
mod write_through;

pub(crate) use backfill::backfill_shared_history;
use coalesce::stale_state_labels_for;
pub use driver::*;
pub(crate) use outbox::persist_outbox_to_disk;
pub use outbox::{default_outbox_path, load_outbox_from_disk};
pub use port::*;
pub use race_window::*;
pub use status::{DroppedEvent, SyncStatus, is_stale, is_startup_fail, staleness_threshold_secs};
pub use types::*;

pub trait OutboxRepo: Send + Sync {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, persist::PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), persist::PersistError>;
}

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

    pub async fn reconcile_dual_state_labels(&self) -> Result<usize, SyncError> {
        let stories = self.read_all_stories();
        let iterations = self.read_iterations();
        let mut repairs = 0usize;
        for story in stories {
            let labels = match self.adapter.fetch_story_labels(story.id.value()).await {
                Ok(l) => l,
                Err(_) => continue,
            };
            let fine = self.read_story_fine_stage(&story);
            let stale = stale_state_labels_for(&*self.codec, &story, &labels, fine, &iterations);
            for stale_label in stale {
                let meta = product_management::story::events::EventMeta {
                    actor: "bpmux:reconcile".to_string(),
                    actor_id: None,
                    ts: Utc::now(),
                };
                self.mutate(StoryEvent::new(
                    meta,
                    story.id,
                    StoryEventKind::TagRemoved { label: stale_label },
                ))?;
                repairs += 1;
            }
        }
        if repairs > 0 {
            self.drain_pass().await?;
        }
        Ok(repairs)
    }
}

mod builder;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod orchestrator_tests;
#[cfg(test)]
mod outbox_capture_tests;
#[cfg(test)]
mod outbox_format_tests;
