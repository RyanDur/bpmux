use std::sync::Arc;

use room::LivePtyRegistry;
use store::Store;

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
        transport::ws::dispatch_roster_leave(store, &routing_id).await;
    }
    for identity in hub.roster_snapshot() {
        let routing_id = super::startup_routing_id(&select, &identity);
        transport::ws::dispatch_roster_join(store, &routing_id, &identity).await;
    }
}

pub(super) async fn start_polling(store: &Arc<Store>, has_refresh: bool) {
    store
        .dispatch(store::AppAction::Ci(store::CiAction::StartPolling))
        .await;
    if has_refresh {
        store
            .dispatch(store::AppAction::Sync(
                store::SyncAction::StartIngestPolling,
            ))
            .await;
        store
            .dispatch(store::AppAction::Sync(
                store::SyncAction::StartHydrationPolling,
            ))
            .await;
    }
}
