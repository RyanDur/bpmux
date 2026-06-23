use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::RefreshDriver;
use crate::session;
use crate::transport;

pub type TestRefreshHandle = Option<Arc<dyn RefreshDriver>>;

fn startup_routing_id(
    routing: &crate::session::routing_identity::RoutingStore,
    identity: &str,
) -> String {
    routing
        .resolve(crate::transport::ws::short_name(identity))
        .into_iter()
        .next()
        .unwrap_or_else(|| identity.to_string())
}

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let auth_state = build_auth_state()?;

    let tracker: transport::rest::TrackerHandle =
        crate::board::adapters::github::GitHubSourceAdapter::from_env().map(std::sync::Arc::new);

    let (action_tx, action_rx) =
        tokio::sync::mpsc::unbounded_channel::<crate::orchestrator::OrchestratorAction>();

    let ci_state_store = crate::transport::ci_state_store::CiStateStore::new();
    let race_window_barrier = race_window_barrier_from_env();

    let test_refresh: TestRefreshHandle = if test_mode_enabled() {
        eprintln!(
            "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
        );
        Some(
            build_test_orchestrator(
                session.clone(),
                Some(action_tx.clone()),
                ci_state_store.clone(),
                race_window_barrier.clone(),
            )
            .await,
        )
    } else if let Some(tracker_arc) = tracker.as_ref().cloned() {
        Some(build_prod_orchestrator(
            session.clone(),
            tracker_arc,
            Some(action_tx.clone()),
            ci_state_store.clone(),
            race_window_barrier.clone(),
        ))
    } else {
        None
    };

    if let Some(refresh) = test_refresh.as_ref().cloned() {
        tokio::spawn(crate::bridge::run_drain_loop(
            refresh.clone(),
            session.clone(),
            crate::bridge::drain_interval_from_env(),
        ));
    }
    let ci_log_store = crate::transport::ci_log_store::CiLogStore::new();
    let electron_capture_lock: std::sync::Arc<tokio::sync::Mutex<()>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(()));
    let pty_registry = std::sync::Arc::new(crate::room::LivePtyRegistry::new());
    let routing_store_dir = session
        .dir
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| session.dir.clone());
    let routing_store = std::sync::Arc::new(crate::session::routing_identity::RoutingStore::load(
        routing_store_dir,
    ));
    let commitment_store = std::sync::Arc::new(crate::room::CommitmentStore::new());
    crate::room::spawn_terminal_delivery_loop(
        session.clone(),
        pty_registry.clone(),
        routing_store.clone(),
        commitment_store.clone(),
    );
    let mut orchestrator_inner = crate::orchestrator::Orchestrator::new(
        session.clone(),
        tracker.as_ref().cloned(),
        ci_state_store.clone(),
        ct.clone(),
    );
    if let Some(refresh) = test_refresh.as_ref().cloned() {
        orchestrator_inner = orchestrator_inner.with_refresh(
            refresh,
            crate::bridge::poll_interval_from_env(),
            crate::bridge::hydration_interval_from_env(),
        );
    }
    let orchestrator = std::sync::Arc::new(orchestrator_inner);
    orchestrator.spawn_action_drainer(action_rx);
    wire_orchestrator_listeners(&orchestrator, &session, &ci_state_store);
    wire_startup_roster(&orchestrator, &session, &pty_registry, &routing_store).await;
    if let Some(refresh) = test_refresh.as_ref().cloned() {
        spawn_owner_park_loop(refresh, orchestrator.clone(), ct.clone());
    }
    start_polling(&orchestrator, test_refresh.is_some()).await;

    let unfurl_cache: transport::unfurl::UnfurlCacheHandle =
        std::sync::Arc::new(transport::unfurl::UnfurlCache::new());

    let router = build_router(RouterDeps {
        auth_state,
        unfurl_cache,
        session,
        tracker,
        ci_state_store,
        ci_log_store,
        electron_capture_lock,
        test_refresh,
        orchestrator,
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
            crate::session::diag::log_diag(
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
mod wiring;
use routes::{gated_routes, test_routes, test_seed_routes};
use setup::*;
use startup_sequence::*;
use wiring::*;
