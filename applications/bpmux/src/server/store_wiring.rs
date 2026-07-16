use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::hub;
use crate::store::{
    AppState, BoardPlacementPersister, BoardPlacementReconciler, CiBroadcast, CiListeners,
    CiPollFetch, EvidencePersister, LocalHistoryPersister, OwnerParkListener, ReactionsBroadcast,
    ReactionsListeners, ReactionsPersister, RosterBroadcast, RosterListeners, RoutingPersister,
    SharedHistoryPersister, Store, StoreParts, app_reduce, ci,
};
use crate::sync::{self, RefreshDriver};
use crate::transport::rest::broadcast_listeners::{
    EpicBroadcast, IterationBroadcast, OutcomeBroadcast, StoryBroadcast,
};
use crate::transport::state_delta_broadcasters::{
    CiHubBroadcaster, ReactionsHubBroadcaster, RosterHubBroadcaster,
};

pub fn build_store(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    parent_ct: CancellationToken,
) -> Store {
    build_store_with_outbox(hub, tracker, parent_ct, Vec::new())
}

pub(crate) fn build_store_with_outbox(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    parent_ct: CancellationToken,
    initial_outbox: Vec<source_sync::QueuedWrite>,
) -> Store {
    Store::from_parts(assemble(hub, tracker, parent_ct, initial_outbox))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_store_with_refresh(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    parent_ct: CancellationToken,
    initial_outbox: Vec<source_sync::QueuedWrite>,
    refresh: Arc<dyn RefreshDriver>,
    ingest_interval: Duration,
    hydration_interval: Duration,
) -> Store {
    let parts = assemble(hub, tracker, parent_ct.clone(), initial_outbox);
    let store_select = crate::store::Select::new(std::sync::Arc::downgrade(&parts.app_store));
    let sync_poll = Arc::new(crate::bridge::polling::SyncPoll::new(
        refresh.clone(),
        store_select.clone(),
        parts.rate_limit_observer.clone(),
        ingest_interval,
        hydration_interval,
        parent_ct.clone(),
    ));
    parts.app_store.add_listener(sync_poll);
    parts.app_store.add_listener(Arc::new(OutcomeBroadcast::new(
        parts.hub.clone(),
        store_select.clone(),
    )));
    parts
        .app_store
        .add_listener(Arc::new(IterationBroadcast::new(
            parts.hub.clone(),
            store_select.clone(),
        )));
    parts.app_store.add_listener(Arc::new(EpicBroadcast::new(
        parts.hub.clone(),
        store_select.clone(),
    )));
    parts.app_store.add_listener(Arc::new(StoryBroadcast::new(
        parts.hub.clone(),
        store_select.clone(),
    )));
    parts.app_store.add_listener(Arc::new(OwnerParkListener));
    parts
        .app_store
        .add_listener(Arc::new(BoardPlacementReconciler));
    parts
        .app_store
        .add_listener(Arc::new(BoardPlacementPersister::new(parts.hub.clone())));
    parts
        .app_store
        .add_listener(Arc::new(ReactionsPersister::new(parts.hub.clone())));
    parts
        .app_store
        .add_listener(Arc::new(EvidencePersister::new(parts.hub.clone())));
    parts
        .app_store
        .add_listener(Arc::new(SharedHistoryPersister::new(parts.hub.clone())));
    parts
        .app_store
        .add_listener(Arc::new(LocalHistoryPersister::new(parts.hub.clone())));
    parts
        .app_store
        .add_listener(Arc::new(sync::OutboxPersister::new(
            sync::OutboxSink::Storage(parts.hub.storage.clone()),
        )));
    parts
        .app_store
        .add_listener(Arc::new(sync::GithubDrain::new(refresh, parent_ct)));
    Store::from_parts(parts)
}

fn assemble(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    parent_ct: CancellationToken,
    initial_outbox: Vec<source_sync::QueuedWrite>,
) -> StoreParts {
    let rate_limit_observer = tracker
        .as_ref()
        .map(|t| t.rate_limit_observer())
        .unwrap_or_default();
    let app_store = kernel::Store::new(
        AppState::with_board(board::persistence::load(&hub.dir))
            .with_reactions(crate::store::reactions::from_store(&reactions::load(
                &hub.dir,
            )))
            .with_evidence(product_management::evidence::from_store(
                &product_management::evidence::load(&hub.dir),
            ))
            .with_shared_history(product_management::history_persistence::load(&hub.dir))
            .with_local_history(board::history_persistence::load(&hub.dir))
            .with_routing(crate::store::routing::from_users(
                &hub.storage.load_users().unwrap_or_default(),
            ))
            .with_ci(super::setup::boot_ci_state())
            .with_outbox(initial_outbox),
        Box::new(app_reduce),
    );
    let store_select = crate::store::Select::new(Arc::downgrade(&app_store));
    let roster_listeners: RosterListeners = Arc::new(Mutex::new(vec![Arc::new(
        RosterHubBroadcaster::new(hub.clone(), store_select),
    )]));
    let reactions_listeners: ReactionsListeners = Arc::new(Mutex::new(vec![Arc::new(
        ReactionsHubBroadcaster::new(hub.clone()),
    )]));
    let ci_listeners: CiListeners = Arc::new(Mutex::new(vec![Arc::new(CiHubBroadcaster::new(
        hub.clone(),
    ))]));
    app_store.add_listener(Arc::new(RosterBroadcast::new(roster_listeners.clone())));
    app_store.add_listener(Arc::new(ReactionsBroadcast::new(
        reactions_listeners.clone(),
    )));
    app_store.add_listener(Arc::new(CiBroadcast::new(ci_listeners.clone())));
    let ci_poll = Arc::new(ci::CiPoll::new(
        tracker.clone(),
        hub.clone(),
        parent_ct,
        rate_limit_observer.clone(),
        Arc::new(crate::transport::ci_watcher::CiWatcherDriver),
    ));
    app_store.add_listener(ci_poll);
    app_store.add_listener(Arc::new(RoutingPersister::new(hub.storage.clone())));
    StoreParts {
        hub,
        rate_limit_observer,
        app_store,
        #[cfg(test)]
        ci_listeners,
        #[cfg(test)]
        roster_listeners,
        #[cfg(test)]
        reactions_listeners,
    }
}
