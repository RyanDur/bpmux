use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use sync::OrchestratorHandle;

fn startup_routing_id(select: &store::Select, identity: &str) -> String {
    select
        .select(|state| store::routing::resolve(&state.routing, user_routing::short_name(identity)))
        .into_iter()
        .next()
        .unwrap_or_else(|| identity.to_string())
}

pub async fn run_server(
    hub: Arc<hub::Hub>,
    data_gateways: storage::DataGateways,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let auth_state = build_auth_state()?;

    let tracker: transport::rest::TrackerHandle =
        integrations::github::GitHubSourceAdapter::from_env().map(std::sync::Arc::new);

    let race_window = race_window_barrier_from_env();

    let (store_inner, orchestrator) =
        compose::compose_store(hub.clone(), data_gateways.clone(), &tracker, ct.clone());
    let store = std::sync::Arc::new(store_inner);

    let ci_log_store = ci::CiLogStore::new();
    let electron_capture_lock: std::sync::Arc<tokio::sync::Mutex<()>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(()));
    let pty_registry = std::sync::Arc::new(room::LivePtyRegistry::new());
    room::spawn_terminal_delivery_loop(
        hub.clone(),
        pty_registry.clone(),
        store.selector(),
        store.dispatcher(),
    );
    if let Some(refresh) = orchestrator.as_ref() {
        tokio::spawn(bridge::run_sync_status_heartbeat(
            refresh.clone(),
            hub.clone(),
            bridge::sync_status_heartbeat_interval_from_env(),
        ));
        startup_tasks::spawn_background_ingest(
            refresh.clone(),
            store.dispatcher(),
            store.selector(),
        );
    }
    wire_startup_roster(&store, &hub, &pty_registry).await;
    start_polling(&store, orchestrator.is_some()).await;

    let unfurl_cache: transport::unfurl::UnfurlCacheHandle =
        std::sync::Arc::new(transport::unfurl::UnfurlCache::new());

    let orchestrator_handle: OrchestratorHandle = orchestrator.map(|orch| {
        let handle: Arc<sync::SyncOrchestrator> = orch;
        handle
    });

    let router = build_router(RouterDeps {
        auth_state,
        unfurl_cache,
        hub,
        tracker,
        ci_log_store,
        electron_capture_lock,
        orchestrator: orchestrator_handle,
        race_window,
        dispatch: store.dispatcher(),
        store,
        pty_registry,
        lessons: data_gateways.lessons.clone(),
        transitions: data_gateways.transitions.clone(),
        ci_runs: data_gateways.ci_runs.clone(),
        port,
    });
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    let shutdown_ct = ct.clone();
    let serve = axum::serve(listener, router)
        .with_graceful_shutdown(async move { shutdown_ct.cancelled_owned().await });
    tokio::select! {
        res = serve => res?,
        _ = async {
            ct.cancelled().await;
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        } => {
            diag::log_diag(
                "server: graceful shutdown deadline (1500ms) elapsed; forcing exit",
            );
        }
    }
    Ok(())
}

pub mod awareness_host;
#[cfg(test)]
mod awareness_host_tests;
mod compose;
mod routes;
mod setup;
mod startup_sequence;
mod startup_tasks;
#[cfg(test)]
mod startup_tasks_tests;
pub mod store_wiring;
mod wiring;
use routes::{gated_routes, test_routes, test_seed_routes};
use setup::*;
use startup_sequence::*;
use wiring::*;

mod daemon;
pub use daemon::run;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod store_tests {
    mod ci_tests;
    mod listeners_tests;
    mod pm_slice_tests;
    mod stores_tests;
}

#[cfg(test)]
mod ci_tests;
#[cfg(test)]
mod local_url_lock_tests;
#[cfg(test)]
mod order_state_pin_tests;
#[cfg(test)]
mod pm_integration_tests;
#[cfg(test)]
mod storage_tests;
#[cfg(test)]
mod sync_drain_listener_tests;
#[cfg(test)]
mod team_roster_tests;
#[cfg(test)]
mod tracker_contract_tests;
#[cfg(test)]
mod transport_excerpt_enrichment_tests;
#[cfg(test)]
mod transport_presence_endpoint_tests;
#[cfg(test)]
mod transport_ws_client_handlers_tests;
#[cfg(test)]
mod transport_ws_roster_projection_tests;
#[cfg(test)]
mod view_integration_tests;
