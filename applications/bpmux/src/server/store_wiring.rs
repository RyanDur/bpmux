use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::hub;
use crate::store::{
    AppState, BoardPlacementPersister, BoardPlacementReconciler, CiBroadcast, CiListeners,
    CiPollFetch, EvidencePersister, LocalHistoryPersister, OwnerParkListener, ReactionsBroadcast,
    ReactionsListeners, ReactionsPersister, RosterBroadcast, RosterListeners, RoutingPersister,
    SharedHistoryPersister, Store, StoreParts, app_reduce, ci, sync,
};
use crate::sync::RefreshDriver;
use crate::transport::rest::broadcast_listeners::{
    EpicBroadcast, IterationBroadcast, OutcomeBroadcast, StoryBroadcast,
};

pub fn build_store(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
) -> Store {
    build_store_with_outbox(hub, tracker, ci_state_store, parent_ct, Vec::new())
}

pub(crate) fn build_store_with_outbox(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    initial_outbox: Vec<crate::sync::QueuedWrite>,
) -> Store {
    Store::from_parts(assemble(
        hub,
        tracker,
        ci_state_store,
        parent_ct,
        initial_outbox,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_store_with_refresh(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    initial_outbox: Vec<crate::sync::QueuedWrite>,
    refresh: Arc<dyn RefreshDriver>,
    ingest_interval: Duration,
    hydration_interval: Duration,
) -> Store {
    let mut parts = assemble(
        hub,
        tracker,
        ci_state_store,
        parent_ct.clone(),
        initial_outbox,
    );
    let sync_poll = Arc::new(crate::bridge::polling::SyncPoll::new(
        refresh.clone(),
        parts.rate_limit_observer.clone(),
        ingest_interval,
        hydration_interval,
        parent_ct.clone(),
    ));
    parts.app_store.add_listener(sync_poll.clone());
    parts.app_store.add_listener(Arc::new(OutcomeBroadcast::new(
        parts.hub.clone(),
        refresh.clone(),
    )));
    parts
        .app_store
        .add_listener(Arc::new(IterationBroadcast::new(
            parts.hub.clone(),
            refresh.clone(),
        )));
    parts.app_store.add_listener(Arc::new(EpicBroadcast::new(
        parts.hub.clone(),
        refresh.clone(),
    )));
    parts.app_store.add_listener(Arc::new(StoryBroadcast::new(
        parts.hub.clone(),
        refresh.clone(),
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
    parts.sync_poll = Some(sync_poll);
    Store::from_parts(parts)
}

fn assemble(
    hub: Arc<hub::Hub>,
    tracker: Option<Arc<dyn CiPollFetch>>,
    ci_state_store: CiStateStore,
    parent_ct: CancellationToken,
    initial_outbox: Vec<crate::sync::QueuedWrite>,
) -> StoreParts {
    let rate_limit_observer = tracker
        .as_ref()
        .map(|t| t.rate_limit_observer())
        .unwrap_or_default();
    let roster_listeners: RosterListeners = Arc::new(Mutex::new(Vec::new()));
    let reactions_listeners: ReactionsListeners = Arc::new(Mutex::new(Vec::new()));
    let ci_listeners: CiListeners = Arc::new(Mutex::new(Vec::new()));
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
            .with_outbox(initial_outbox),
        Box::new(app_reduce),
    );
    app_store.add_listener(Arc::new(RosterBroadcast::new(roster_listeners.clone())));
    app_store.add_listener(Arc::new(ReactionsBroadcast::new(
        reactions_listeners.clone(),
    )));
    app_store.add_listener(Arc::new(CiBroadcast::new(ci_listeners.clone())));
    let ci_poll = Arc::new(ci::CiPoll::new(
        tracker.clone(),
        ci_state_store.clone(),
        hub.clone(),
        parent_ct,
        rate_limit_observer.clone(),
        Arc::new(crate::transport::ci_watcher::CiWatcherDriver),
    ));
    app_store.add_listener(ci_poll.clone());
    app_store.add_listener(Arc::new(RoutingPersister::new(hub.storage.clone())));
    StoreParts {
        hub,
        rate_limit_observer,
        ci_poll,
        sync_poll: None,
        app_store,
        ci_listeners,
        roster_listeners,
        reactions_listeners,
    }
}
