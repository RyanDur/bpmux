pub mod adapters;
pub mod column;
pub mod events;
pub mod label_codec;
pub mod orchestrator;
pub mod peer;
pub mod ready_order_label;
pub mod store;
pub mod sync;
pub mod velocity;
pub mod view_model;

#[cfg(test)]
mod canonical_tests;
#[cfg(test)]
mod conflict_classifier_tests;
#[cfg(test)]
mod event_messages_audit_tests;
#[cfg(test)]
mod orchestrator_tests;
#[cfg(test)]
mod ready_order_label_codec;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod story_view_projection;
#[cfg(test)]
mod velocity_tests;

pub use events::BoardEvent;
pub use orchestrator::{
    DrainOneOutcome, MutationId, QueuedEvent, RaceWindowBarrier, RefreshDriver, SyncOrchestrator,
    SyncStatus, SyncTransition, acknowledge_dropped, default_outbox_path, sync_transition,
};
pub use peer::PeerId;
pub use store::{BoardStore, PersistError};
pub use sync::{SourceAdapter, SyncError};
