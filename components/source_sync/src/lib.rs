pub mod db;
pub mod store;
use std::collections::VecDeque;

pub mod health;
pub mod queue;

mod port;
mod provenance;
mod status;
mod types;

pub use port::{
    BoxedFuture, ConditionalBody, Ingested, PushContext, SourceAdapter, StoryWithPlacement,
    SyncError, classify_close_error, is_permanent_push_failure,
};
pub use provenance::cache_fill_meta;
pub use status::{DroppedEvent, SyncStatus, is_stale, is_startup_fail, staleness_threshold_secs};
pub use types::{MutationId, PendingWrite, QueuedEvent, QueuedWrite};

pub trait OutboxRepo: Send + Sync {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, persist::PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), persist::PersistError>;
}
