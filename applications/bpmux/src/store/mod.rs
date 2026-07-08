use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::hub;
use crate::kernel;

pub use crate::board;
pub use crate::ci::slice as ci;
pub use crate::reactions::slice as reactions;
pub use crate::roster;
pub mod commitments;
pub mod sync;
pub mod sync_health;

mod action;
mod app;
mod dispatch;
mod select;
mod slice_access;
mod snapshot;
mod wiring;

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
    parent_ct: CancellationToken,
    rate_limit_observer: crate::github::RateLimitObserver,
    ci_poll: Arc<ci::CiPoll>,
    sync_poll: Option<Arc<sync::SyncPoll>>,
    github_drain: Option<Arc<sync::GithubDrain>>,
    app_store: Arc<kernel::Store<app::AppState, app::AppAction>>,
    ci_listeners: app::CiListeners,
    roster_listeners: app::RosterListeners,
    reactions_listeners: app::ReactionsListeners,
}

impl Store {
    pub fn rate_limit_observer(&self) -> &crate::github::RateLimitObserver {
        &self.rate_limit_observer
    }

    pub fn hub(&self) -> Arc<hub::Hub> {
        Arc::clone(&self.hub)
    }
}
