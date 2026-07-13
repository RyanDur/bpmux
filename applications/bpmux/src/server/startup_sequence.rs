use std::sync::Arc;

use crate::ci::CiStateStore;
use crate::hub;
use crate::room::LivePtyRegistry;
use crate::store::Store;

pub(super) fn wire_store_listeners(
    store: &Arc<Store>,
    hub: &Arc<hub::Hub>,
    ci_state_store: &CiStateStore,
) {
    store.add_ci_listener(Arc::new(crate::store::CiStateStoreWriter::new(
        ci_state_store.clone(),
    )));
    store.add_ci_listener(Arc::new(
        crate::transport::state_delta_broadcasters::CiHubBroadcaster::new(hub.clone()),
    ));
    store.add_reactions_listener(Arc::new(
        crate::transport::state_delta_broadcasters::ReactionsHubBroadcaster::new(hub.clone()),
    ));
    store.add_roster_listener(Arc::new(
        crate::transport::state_delta_broadcasters::RosterHubBroadcaster::new(
            hub.clone(),
            store.selector(),
        ),
    ));
}

pub(super) async fn wire_startup_roster(
    store: &Arc<Store>,
    hub: &Arc<hub::Hub>,
    pty_registry: &Arc<LivePtyRegistry>,
) {
    let select = store.selector();
    let live_terminal_ids = pty_registry.reap_dead_and_list_live();
    let reaped_ghosts = hub.reconcile_roster(&live_terminal_ids);
    for identity in &reaped_ghosts {
        let routing_id = super::startup_routing_id(&select, identity);
        crate::transport::ws::dispatch_roster_leave(store, &routing_id).await;
    }
    for identity in hub.roster_snapshot() {
        let routing_id = super::startup_routing_id(&select, &identity);
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
