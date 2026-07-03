pub mod adapters;
pub mod arrangement;
pub use crate::product_management::story::events;
pub mod label_codec;
pub use crate::sync as orchestrator;
pub mod peer;
pub mod ready_order_label;
pub mod sync;
pub mod velocity;
pub mod view_model;

#[cfg(test)]
mod canonical_tests;
#[cfg(test)]
mod event_messages_audit_tests;
#[cfg(test)]
mod peer_tests;
#[cfg(test)]
mod ready_order_label_codec;
#[cfg(test)]
mod story_view_projection;
#[cfg(test)]
mod velocity_tests;

pub use events::StoryEvent;
pub use orchestrator::PersistError;
pub use orchestrator::{
    DrainOneOutcome, MutationId, QueuedEvent, RaceWindowBarrier, RefreshDriver, SyncOrchestrator,
    SyncStatus, SyncTransition, acknowledge_dropped, default_outbox_path, sync_transition,
};
pub use peer::PeerId;
pub use sync::{SourceAdapter, SyncError};
