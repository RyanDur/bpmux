use std::sync::Arc;

use crate::hub;

pub use crate::ci::slice as ci;
pub use crate::reactions::slice as reactions;
pub use crate::roster;
pub use board;
pub use user_routing::slice as routing;
pub mod commitments;
pub mod sync;
pub mod sync_health;

mod action;
mod app;
mod dispatch;
mod select;
mod slice_access;
mod snapshot;

#[cfg(test)]
mod ci_tests;
#[cfg(test)]
mod listeners_tests;
#[cfg(test)]
mod stores_tests;
#[cfg(test)]
pub(crate) mod test_support;

pub use action::PollKind;
pub use app::{AppAction, AppState, app_reduce};
pub(crate) use app::{
    BoardPlacementPersister, BoardPlacementReconciler, CiBroadcast, CiListeners, EvidencePersister,
    LocalHistoryPersister, OwnerParkListener, ReactionsBroadcast, ReactionsListeners,
    ReactionsPersister, RosterBroadcast, RosterListeners, RoutingPersister, SharedHistoryPersister,
};
pub use ci::{
    BoxedFuture, CiAction, CiBroadcastListener, CiHubBroadcaster, CiPollFetch, CiSnapshot,
    CiStateStoreWriter,
};
pub use commitments::CommitmentsAction;
pub use dispatch::Dispatch;
pub use reactions::{
    AggregatedReaction, ReactionsAction, ReactionsBroadcastListener, ReactionsHubBroadcaster,
};
pub use roster::{
    Presence, RosterAction, RosterBroadcastListener, RosterHubBroadcaster, RosterMember, RosterRole,
};
pub use select::Select;
#[cfg(test)]
pub(crate) use select::{SeededStore, bind_seeded_refresh};
pub use sync::SyncAction;
pub use sync_health::SyncHealthAction;

pub struct Store {
    hub: Arc<hub::Hub>,
    rate_limit_observer: crate::rate_limit::RateLimitObserver,
    ci_poll: Arc<ci::CiPoll>,
    sync_poll: Option<Arc<crate::bridge::polling::SyncPoll>>,
    app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    ci_listeners: app::CiListeners,
    roster_listeners: app::RosterListeners,
    reactions_listeners: app::ReactionsListeners,
}

pub(crate) struct StoreParts {
    pub(crate) hub: Arc<hub::Hub>,
    pub(crate) rate_limit_observer: crate::rate_limit::RateLimitObserver,
    pub(crate) ci_poll: Arc<ci::CiPoll>,
    pub(crate) sync_poll: Option<Arc<crate::bridge::polling::SyncPoll>>,
    pub(crate) app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    pub(crate) ci_listeners: app::CiListeners,
    pub(crate) roster_listeners: app::RosterListeners,
    pub(crate) reactions_listeners: app::ReactionsListeners,
}

impl Store {
    pub(crate) fn from_parts(parts: StoreParts) -> Self {
        Self {
            hub: parts.hub,
            rate_limit_observer: parts.rate_limit_observer,
            ci_poll: parts.ci_poll,
            sync_poll: parts.sync_poll,
            app_store: parts.app_store,
            ci_listeners: parts.ci_listeners,
            roster_listeners: parts.roster_listeners,
            reactions_listeners: parts.reactions_listeners,
        }
    }

    pub fn rate_limit_observer(&self) -> &crate::rate_limit::RateLimitObserver {
        &self.rate_limit_observer
    }

    pub fn hub(&self) -> Arc<hub::Hub> {
        Arc::clone(&self.hub)
    }
}
