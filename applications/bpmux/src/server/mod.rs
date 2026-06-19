use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::{BoardStore, RefreshDriver};
use crate::session;
use crate::transport;
use crate::transport::auth::AuthState;

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

fn spawn_periodic_snapshot(
    store: Arc<BoardStore>,
    storage: Arc<dyn crate::session::storage::Storage>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if !store.is_dirty() {
                continue;
            }
            if let Err(e) = store.persist_to_storage(storage.as_ref()) {
                crate::session::diag::log_diag(&format!(
                    "server: periodic board-store snapshot failed: {e}"
                ));
            }
        }
    });
}

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let token_path = session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot locate ~/.bpmux auth token path"))?;
    let token = session::auth_token::ensure(&token_path).map_err(|e| {
        anyhow::anyhow!(
            "Unable to write auth token to {}: {e}",
            token_path.display()
        )
    })?;
    let auth_state = AuthState::new(token);

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
        Some(
            build_prod_orchestrator(
                session.clone(),
                tracker_arc,
                Some(action_tx.clone()),
                ci_state_store.clone(),
                race_window_barrier.clone(),
            )
            .await,
        )
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
    crate::room::spawn_terminal_delivery_loop(
        session.clone(),
        pty_registry.clone(),
        routing_store.clone(),
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
    orchestrator.add_ci_listener(std::sync::Arc::new(
        crate::orchestrator::CiStateStoreWriter::new(ci_state_store.clone()),
    ));
    orchestrator.add_ci_listener(std::sync::Arc::new(
        crate::orchestrator::CiSessionBroadcaster::new(session.clone()),
    ));
    orchestrator.add_reactions_listener(std::sync::Arc::new(
        crate::orchestrator::ReactionsSessionBroadcaster::new(session.clone()),
    ));
    orchestrator.add_roster_listener(std::sync::Arc::new(
        crate::orchestrator::RosterSessionBroadcaster::new(session.clone()),
    ));
    let live_terminal_ids = pty_registry.reap_dead_and_list_live();
    let reaped_ghosts = session.reconcile_roster(&live_terminal_ids);
    for identity in &reaped_ghosts {
        let routing_id = startup_routing_id(&routing_store, identity);
        crate::transport::ws::dispatch_roster_leave(&orchestrator, &routing_id).await;
    }
    for identity in session.roster_snapshot() {
        let routing_id = startup_routing_id(&routing_store, &identity);
        crate::transport::ws::dispatch_roster_join(&orchestrator, &routing_id, &identity).await;
    }
    if let Some(refresh) = test_refresh.as_ref().cloned() {
        let owner_park =
            crate::orchestrator::OwnerParkBroadcaster::new(orchestrator.clone(), refresh.store());
        let mut events = refresh.store().subscribe();
        let owner_park_ct = ct.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = owner_park_ct.cancelled() => return,
                    received = events.recv() => match received {
                        Ok(event) => owner_park.on_board_event(&event).await,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            owner_park.recompute_from_snapshot().await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        });
    }
    orchestrator
        .dispatch(crate::orchestrator::OrchestratorAction::StartCiPolling)
        .await;
    if test_refresh.is_some() {
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::StartIngestPolling)
            .await;
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::StartHydrationPolling)
            .await;
    }

    let unfurl_cache: transport::unfurl::UnfurlCacheHandle =
        std::sync::Arc::new(transport::unfurl::UnfurlCache::new());

    let gated_routes = gated_routes(auth_state, unfurl_cache);
    let test_seed_routes = test_seed_routes();
    let test_routes = test_routes(test_refresh.is_some());

    let router = axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::get(transport::rest::get_messages).post(transport::rest::post_message),
        )
        .route("/api/board", axum::routing::get(transport::rest::get_board))
        .route(
            "/api/wait-for-event",
            axum::routing::get(transport::rest::wait_for_event),
        )
        .route(
            "/api/search",
            axum::routing::get(transport::rest::search_messages),
        )
        .route(
            "/api/lessons",
            axum::routing::get(transport::rest::get_lessons).post(transport::rest::post_lesson),
        )
        .route(
            "/api/peer-lessons",
            axum::routing::get(transport::rest::get_peer_lessons),
        )
        .route(
            "/api/classifier-misfire",
            axum::routing::post(transport::rest::post_classifier_misfire),
        )
        .route(
            "/api/register",
            axum::routing::post(transport::rest::post_register),
        )
        .merge(gated_routes)
        .merge(test_routes)
        .merge(test_seed_routes)
        .with_state(session)
        .layer(axum::Extension(tracker))
        .layer(axum::Extension(ci_state_store))
        .layer(axum::Extension(ci_log_store))
        .layer(axum::Extension(electron_capture_lock))
        .layer(axum::Extension(test_refresh))
        .layer(axum::Extension(orchestrator.clone()))
        .layer(axum::Extension(pty_registry.clone()))
        .layer(axum::Extension(routing_store.clone()))
        .layer(axum::Extension(port))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        );
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
use routes::{gated_routes, test_routes, test_seed_routes};
use setup::*;
