use std::sync::Arc;

use crate::ci::CiStateStore;
use crate::room::LivePtyRegistry;
use crate::routing::RoutingStore;
use crate::session;
use crate::store::Store;

pub(super) fn wire_store_listeners(
    store: &Arc<Store>,
    session: &Arc<session::Session>,
    ci_state_store: &CiStateStore,
) {
    store.add_ci_listener(Arc::new(crate::store::CiStateStoreWriter::new(
        ci_state_store.clone(),
    )));
    store.add_ci_listener(Arc::new(crate::store::CiSessionBroadcaster::new(
        session.clone(),
    )));
    store.add_reactions_listener(Arc::new(crate::store::ReactionsSessionBroadcaster::new(
        session.clone(),
    )));
    store.add_roster_listener(Arc::new(crate::store::RosterSessionBroadcaster::new(
        session.clone(),
    )));
}

pub(super) async fn wire_startup_roster(
    store: &Arc<Store>,
    session: &Arc<session::Session>,
    pty_registry: &Arc<LivePtyRegistry>,
    routing_store: &Arc<RoutingStore>,
) {
    let live_terminal_ids = pty_registry.reap_dead_and_list_live();
    let reaped_ghosts = session.reconcile_roster(&live_terminal_ids);
    for identity in &reaped_ghosts {
        let routing_id = super::startup_routing_id(routing_store, identity);
        crate::transport::ws::dispatch_roster_leave(store, &routing_id).await;
    }
    for identity in session.roster_snapshot() {
        let routing_id = super::startup_routing_id(routing_store, &identity);
        crate::transport::ws::dispatch_roster_join(store, &routing_id, &identity).await;
    }
}

pub(super) async fn start_polling(store: &Arc<Store>, has_refresh: bool) {
    store
        .dispatch(crate::store::AppAction::Ci(
            crate::store::CiAction::StartPolling,
        ))
        .await;
    if has_refresh {
        store
            .dispatch(crate::store::AppAction::Sync(
                crate::store::SyncAction::StartIngestPolling,
            ))
            .await;
        store
            .dispatch(crate::store::AppAction::Sync(
                crate::store::SyncAction::StartHydrationPolling,
            ))
            .await;
    }
}
