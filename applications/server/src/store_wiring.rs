use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use store::{
    AppState, BoardPlacementPersister, BoardPlacementReconciler, CiPollFetch, EvidencePersister,
    LocalHistoryPersister, OwnerParkListener, ReactionsPersister, RoutingPersister,
    SharedHistoryPersister, Store, StoreParts, app_reduce,
};
use sync::{self, SyncOrchestrator};
use transport::rest::broadcast_listeners::BoardBroadcast;
use transport::rest::hydration_listeners::SliceHydration;

pub fn build_store(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    tracker: Option<Arc<dyn CiPollFetch>>,
    parent_ct: CancellationToken,
) -> Store {
    let observer = tracker
        .as_ref()
        .map(|t| t.rate_limit_observer())
        .unwrap_or_default();
    Store::from_parts(assemble(
        hub,
        data_gateways,
        Arc::new(move || tracker.clone()),
        observer,
        parent_ct,
        Vec::new(),
        ci::ActionsUrl::for_repo(None),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_store_with_refresh<A: sync::SourceAdapter + 'static>(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    cell: integrations::github::live::TrackerCell,
    connected: bridge::polling::ConnectedProbe,
    parent_ct: CancellationToken,
    initial_outbox: Vec<source_sync::QueuedWrite>,
    adapter: Arc<A>,
    codec: Arc<dyn product_management::label_codec::LabelCodec + Send + Sync>,
    ingest_interval: Duration,
    hydration_interval: Duration,
    actions_url: ci::ActionsUrl,
) -> (Store, Arc<SyncOrchestrator>) {
    let outbox_sink = data_gateways.outbox.clone();
    let observer = cell.observer();
    let parts = assemble(
        hub,
        data_gateways,
        transport::ci_watch_driver::cell_fetch_resolver(cell),
        observer,
        parent_ct.clone(),
        initial_outbox,
        actions_url,
    );
    let store_select = store::Select::new(std::sync::Arc::downgrade(&parts.app_store));
    parts.app_store.add_listener(Arc::new(SliceHydration::board(
        parts.hub.clone(),
        store_select.clone(),
    )));
    let refresh = Arc::new(SyncOrchestrator::new(
        adapter,
        codec,
        store_select.clone(),
        store::Dispatch::for_app_store(&parts.app_store),
    ));
    let sync_poll = Arc::new(bridge::polling::SyncPoll::new(
        refresh.clone(),
        store_select.clone(),
        parts.rate_limit_observer.clone(),
        ingest_interval,
        hydration_interval,
        parent_ct.clone(),
        connected,
    ));
    parts.app_store.add_listener(sync_poll);
    parts
        .app_store
        .add_listener(Arc::new(BoardBroadcast::new(parts.hub.clone())));
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
            sync::OutboxSink::Storage(outbox_sink),
        )));
    parts
        .app_store
        .add_listener(Arc::new(sync::GithubDrain::new(refresh.clone(), parent_ct)));
    (Store::from_parts(parts), refresh)
}

fn assemble(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    ci_fetch: transport::ci_watch_driver::CiFetchResolver,
    rate_limit_observer: rate_limit::RateLimitObserver,
    parent_ct: CancellationToken,
    initial_outbox: Vec<source_sync::QueuedWrite>,
    actions_url: ci::ActionsUrl,
) -> StoreParts {
    let app_store = kernel::Store::new(
        AppState::with_board(board::persistence::load(&hub.dir))
            .with_reactions(store::reactions::from_store(&reactions::load(&hub.dir)))
            .with_evidence(product_management::evidence::from_store(
                &product_management::evidence::load(&hub.dir),
            ))
            .with_shared_history(product_management::history_persistence::load(&hub.dir))
            .with_local_history(board::history_persistence::load(&hub.dir))
            .with_routing(store::routing::from_users(
                &data_gateways.users.load_users().unwrap_or_default(),
            ))
            .with_ci(super::setup::boot_ci_state())
            .with_outbox(initial_outbox),
        Box::new(app_reduce),
    );
    let store_select = store::Select::new(Arc::downgrade(&app_store));
    let broadcasters =
        super::store_listeners::wire_hub_broadcasters(&app_store, &hub, &actions_url);
    app_store.add_listener(Arc::new(SliceHydration::roster(
        hub.clone(),
        store_select.clone(),
    )));
    app_store.add_listener(Arc::new(SliceHydration::reactions(
        hub.clone(),
        store_select.clone(),
    )));
    app_store.add_listener(Arc::new(SliceHydration::ci(
        hub.clone(),
        store_select,
        actions_url.clone(),
    )));
    let ci_poll = Arc::new(store::ci_watch::CiPoll::new(
        hub.clone(),
        parent_ct,
        rate_limit_observer.clone(),
        Arc::new(transport::ci_watch_driver::CiWatcherDriver {
            actions_url: actions_url.clone(),
            fetch: ci_fetch,
        }),
    ));
    app_store.add_listener(ci_poll);
    app_store.add_listener(Arc::new(RoutingPersister::new(data_gateways.users.clone())));
    app_store.add_listener(Arc::new(store::TransitionPersister::new(
        data_gateways.transitions.clone(),
    )));
    app_store.add_listener(Arc::new(store::CiRunsPersister::new(
        data_gateways.ci_runs.clone(),
    )));
    StoreParts {
        hub,
        rate_limit_observer,
        app_store,
        ci_listeners: broadcasters.ci,
        roster_listeners: broadcasters.roster,
        reactions_listeners: broadcasters.reactions,
    }
}
