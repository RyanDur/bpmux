use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::hub;
use crate::sync::OrchestratorHandle;
use crate::transport;

fn startup_routing_id(routing: &crate::routing::RoutingStore, identity: &str) -> String {
    routing
        .resolve(crate::routing::short_name(identity))
        .into_iter()
        .next()
        .unwrap_or_else(|| identity.to_string())
}

pub async fn run_server(
    hub: Arc<hub::Hub>,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let auth_state = build_auth_state()?;

    let tracker: transport::rest::TrackerHandle =
        crate::integrations::github::GitHubSourceAdapter::from_env().map(std::sync::Arc::new);

    let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel::<crate::store::AppAction>();

    let ci_state_store = crate::ci::CiStateStore::new();
    let race_window_barrier = race_window_barrier_from_env();

    let (orchestrator, hydrated_outbox): (OrchestratorHandle, Vec<crate::sync::QueuedWrite>) =
        if test_mode_enabled() {
            eprintln!(
                "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
            );
            let (orch, writes) = build_test_orchestrator(
                hub.clone(),
                Some(action_tx.clone()),
                ci_state_store.clone(),
                race_window_barrier.clone(),
            )
            .await;
            (Some(orch), writes)
        } else if let Some(tracker_arc) = tracker.as_ref().cloned() {
            let (orch, writes) = build_prod_orchestrator(
                hub.clone(),
                tracker_arc,
                Some(action_tx.clone()),
                ci_state_store.clone(),
                race_window_barrier.clone(),
            );
            (Some(orch), writes)
        } else {
            (None, Vec::new())
        };

    if let Some(refresh) = orchestrator.as_ref().cloned() {
        tokio::spawn(crate::bridge::run_drain_loop(
            refresh.clone(),
            hub.clone(),
            crate::bridge::drain_interval_from_env(),
        ));
    }
    let ci_log_store = crate::transport::ci_log_store::CiLogStore::new();
    let electron_capture_lock: std::sync::Arc<tokio::sync::Mutex<()>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(()));
    let pty_registry = std::sync::Arc::new(crate::room::LivePtyRegistry::new());
    let routing_store = std::sync::Arc::new(crate::routing::RoutingStore::new(hub.storage.clone()));
    let commitment_store = std::sync::Arc::new(crate::room::CommitmentStore::new());
    crate::room::spawn_terminal_delivery_loop(
        hub.clone(),
        pty_registry.clone(),
        routing_store.clone(),
        commitment_store.clone(),
    );
    let mut store_inner = crate::store::Store::with_initial_outbox(
        hub.clone(),
        tracker
            .as_ref()
            .cloned()
            .map(|t| t as std::sync::Arc<dyn crate::store::CiPollFetch>),
        ci_state_store.clone(),
        ct.clone(),
        hydrated_outbox,
    );
    if let Some(refresh) = orchestrator.as_ref().cloned() {
        store_inner = store_inner.with_refresh(
            refresh,
            crate::bridge::poll_interval_from_env(),
            crate::bridge::hydration_interval_from_env(),
        );
    }
    let store = std::sync::Arc::new(store_inner);
    if let Some(refresh) = orchestrator.as_ref() {
        refresh.bind_select(store.selector());
        refresh.bind_story_dispatch(store.story_dispatcher());
        refresh.bind_board_dispatch(store.board_dispatcher());
        startup_tasks::spawn_background_ingest(
            refresh.clone(),
            crate::store::Dispatch::new(action_tx.clone()),
        );
    }
    store.spawn_action_drainer(action_rx);
    wire_store_listeners(&store, &hub, &ci_state_store);
    wire_startup_roster(&store, &hub, &pty_registry, &routing_store).await;
    start_polling(&store, orchestrator.is_some()).await;

    let unfurl_cache: transport::unfurl::UnfurlCacheHandle =
        std::sync::Arc::new(transport::unfurl::UnfurlCache::new());

    let router = build_router(RouterDeps {
        auth_state,
        unfurl_cache,
        hub,
        tracker,
        ci_state_store,
        ci_log_store,
        electron_capture_lock,
        orchestrator,
        store,
        dispatch: crate::store::Dispatch::new(action_tx),
        pty_registry,
        routing_store,
        commitment_store,
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
            crate::diag::log_diag(
                "server: graceful shutdown deadline (1500ms) elapsed; forcing exit",
            );
        }
    }
    Ok(())
}

mod routes;
mod setup;
mod startup_sequence;
mod startup_tasks;
#[cfg(test)]
mod startup_tasks_tests;
mod wiring;
use routes::{gated_routes, test_routes, test_seed_routes};
use setup::*;
use startup_sequence::*;
use wiring::*;
