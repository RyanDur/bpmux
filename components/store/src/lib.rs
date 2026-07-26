use std::sync::Arc;

pub use board;
pub use ci::slice as ci;
pub use reactions::slice as reactions;
pub use roster;
pub use source_sync::health as sync_health;
pub use source_sync::queue as sync;
pub use user_routing::slice as routing;
pub mod ci_watch;
pub mod commitments;

mod action;
mod app;
mod dispatch;
mod select;
mod slice_access;
mod snapshot;

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod sync_reduce_tests;

pub use action::PollKind;
pub use app::{AppAction, AppState, app_reduce};
pub use app::{
    BoardPlacementPersister, BoardPlacementReconciler, CiBroadcast, CiListeners, CiRunsPersister,
    EvidencePersister, LocalHistoryPersister, OwnerParkListener, ReactionsBroadcast,
    ReactionsListeners, ReactionsPersister, RosterBroadcast, RosterListeners, RoutingPersister,
    SharedHistoryPersister, TransitionPersister,
};
pub use ci::{BoxedFuture, CiAction, CiBroadcastListener, CiPollFetch, CiSnapshot};
pub use commitments::CommitmentsAction;
pub use dispatch::Dispatch;
pub use reactions::{AggregatedReaction, ReactionsAction, ReactionsBroadcastListener};
pub use roster::{Presence, RosterAction, RosterBroadcastListener, RosterMember, RosterRole};
pub use select::Select;
pub use sync::SyncAction;
pub use sync_health::SyncHealthAction;

pub struct Store {
    hub: Arc<hub::Hub>,
    rate_limit_observer: rate_limit::RateLimitObserver,
    app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    actions_url: ::ci::ActionsUrl,
    #[cfg(feature = "test-support")]
    ci_listeners: app::CiListeners,
    #[cfg(feature = "test-support")]
    roster_listeners: app::RosterListeners,
    #[cfg(feature = "test-support")]
    reactions_listeners: app::ReactionsListeners,
}

pub struct StoreParts {
    pub hub: Arc<hub::Hub>,
    pub rate_limit_observer: rate_limit::RateLimitObserver,
    pub app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    pub actions_url: ::ci::ActionsUrl,
    pub ci_listeners: app::CiListeners,
    pub roster_listeners: app::RosterListeners,
    pub reactions_listeners: app::ReactionsListeners,
}

impl Store {
    pub fn from_parts(parts: StoreParts) -> Self {
        Self {
            hub: parts.hub,
            rate_limit_observer: parts.rate_limit_observer,
            app_store: parts.app_store,
            actions_url: parts.actions_url,
            #[cfg(feature = "test-support")]
            ci_listeners: parts.ci_listeners,
            #[cfg(feature = "test-support")]
            roster_listeners: parts.roster_listeners,
            #[cfg(feature = "test-support")]
            reactions_listeners: parts.reactions_listeners,
        }
    }

    pub fn actions_url(&self) -> &::ci::ActionsUrl {
        &self.actions_url
    }

    pub fn rate_limit_observer(&self) -> &rate_limit::RateLimitObserver {
        &self.rate_limit_observer
    }

    pub fn hub(&self) -> Arc<hub::Hub> {
        Arc::clone(&self.hub)
    }
}
