use std::collections::VecDeque;

pub mod health;
pub mod queue;

mod status;
mod types;

pub use status::{DroppedEvent, SyncStatus, is_stale, is_startup_fail, staleness_threshold_secs};
pub use types::{MutationId, PendingWrite, QueuedEvent, QueuedWrite};

pub trait OutboxRepo: Send + Sync {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, persist::PersistError>;
    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), persist::PersistError>;
}
