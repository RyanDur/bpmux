pub mod adapters;
pub mod column;
pub mod domain;
pub mod events;
pub mod iteration_label;
pub mod iteration_order_backfill;
pub mod iteration_readiness;
pub mod label_codec;
pub mod orchestrator;
pub mod peer;
pub mod ready_order_label;
pub mod roadmap_drill;
pub mod roadmap_filter;
pub mod roadmap_view;
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
mod iteration_label_tests;
#[cfg(test)]
mod iteration_readiness_tests;
#[cfg(test)]
mod orchestrator_tests;
#[cfg(test)]
mod roadmap_drill_tests;
#[cfg(test)]
mod roadmap_filter_tests;
#[cfg(test)]
mod roadmap_view_tests;
#[cfg(test)]
mod store_tests;

pub use domain::{FreeComment, Gate, GateComment, Story, StoryState};
pub use events::BoardEvent;
pub use orchestrator::{
    DrainOneOutcome, MutationId, QueuedEvent, RaceWindowBarrier, RefreshDriver, SyncOrchestrator,
    SyncStatus, SyncTransition, acknowledge_dropped, default_outbox_path,
    mark_recovering_from_corruption, sync_transition,
};
pub use peer::PeerId;
pub use store::{BoardStore, PersistError};
pub use sync::{SourceAdapter, SyncError};
