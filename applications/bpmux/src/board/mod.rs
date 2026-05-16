pub mod adapters;
pub mod column;
pub mod domain;
pub mod events;
pub mod iteration_label;
pub mod orchestrator;
pub mod peer;
pub mod ready_order_label;
pub mod store;
pub mod sync;

#[cfg(test)]
mod canonical_tests;
#[cfg(test)]
mod conflict_classifier_tests;
#[cfg(test)]
mod domain_tests;
#[cfg(test)]
mod event_messages_audit_tests;
#[cfg(test)]
mod orchestrator_tests;
#[cfg(test)]
mod store_tests;

pub use domain::{FreeComment, Gate, GateComment, Story, StoryState};
pub use events::BoardEvent;
pub use orchestrator::{
    DrainOneOutcome, MutationId, QueuedEvent, RefreshDriver, SyncOrchestrator, SyncStatus,
    acknowledge_dropped, default_outbox_path, mark_recovering_from_corruption,
};
pub use peer::PeerId;
pub use store::{BoardStore, PersistError, default_store_path};
pub use sync::{SourceAdapter, SyncError};
