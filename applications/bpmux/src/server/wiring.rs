use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::RefreshDriver;
use crate::board::adapters::github::GitHubSourceAdapter;
use crate::orchestrator::Orchestrator;
use crate::room::LivePtyRegistry;
use crate::session::{self, routing_identity::RoutingStore};
use crate::transport;
use crate::transport::auth::AuthState;
use crate::transport::ci_log_store::CiLogStore;
use crate::transport::ci_state_store::CiStateStore;

use super::TestRefreshHandle;

type ProdOrchestrator = Orchestrator<GitHubSourceAdapter>;

pub(super) struct RouterDeps {
    pub auth_state: AuthState,
    pub unfurl_cache: transport::unfurl::UnfurlCacheHandle,
    pub session: Arc<session::Session>,
    pub tracker: transport::rest::TrackerHandle,
    pub ci_state_store: CiStateStore,
    pub ci_log_store: CiLogStore,
    pub electron_capture_lock: Arc<tokio::sync::Mutex<()>>,
    pub test_refresh: TestRefreshHandle,
    pub orchestrator: Arc<ProdOrchestrator>,
    pub pty_registry: Arc<LivePtyRegistry>,
    pub routing_store: Arc<RoutingStore>,
    pub port: u16,
}

pub(super) fn build_router(deps: RouterDeps) -> axum::Router {
    let RouterDeps {
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
        port,
    } = deps;
    let has_refresh = test_refresh.is_some();
    let gated_routes = super::gated_routes(auth_state, unfurl_cache);
    let test_seed_routes = super::test_seed_routes();
    let test_routes = super::test_routes(has_refresh);

    axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::get(transport::rest::get_messages).post(transport::rest::post_message),
        )
        .route("/api/board", axum::routing::get(transport::rest::get_board))
        .route(
            "/api/roadmap",
            axum::routing::get(transport::rest::get_roadmap),
        )
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
        .layer(axum::Extension(orchestrator))
        .layer(axum::Extension(pty_registry))
        .layer(axum::Extension(routing_store))
        .layer(axum::Extension(port))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
}

pub(super) fn build_auth_state() -> anyhow::Result<AuthState> {
    let token_path = session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot locate ~/.bpmux auth token path"))?;
    let token = session::auth_token::ensure(&token_path).map_err(|e| {
        anyhow::anyhow!(
            "Unable to write auth token to {}: {e}",
            token_path.display()
        )
    })?;
    Ok(AuthState::new(token))
}

pub(super) fn wire_orchestrator_listeners(
    orchestrator: &Arc<ProdOrchestrator>,
    session: &Arc<session::Session>,
    ci_state_store: &CiStateStore,
) {
    orchestrator.add_ci_listener(Arc::new(crate::orchestrator::CiStateStoreWriter::new(
        ci_state_store.clone(),
    )));
    orchestrator.add_ci_listener(Arc::new(crate::orchestrator::CiSessionBroadcaster::new(
        session.clone(),
    )));
    orchestrator.add_reactions_listener(Arc::new(
        crate::orchestrator::ReactionsSessionBroadcaster::new(session.clone()),
    ));
    orchestrator.add_roster_listener(Arc::new(
        crate::orchestrator::RosterSessionBroadcaster::new(session.clone()),
    ));
}

pub(super) async fn wire_startup_roster(
    orchestrator: &Arc<ProdOrchestrator>,
    session: &Arc<session::Session>,
    pty_registry: &Arc<LivePtyRegistry>,
    routing_store: &Arc<RoutingStore>,
) {
    let live_terminal_ids = pty_registry.reap_dead_and_list_live();
    let reaped_ghosts = session.reconcile_roster(&live_terminal_ids);
    for identity in &reaped_ghosts {
        let routing_id = super::startup_routing_id(routing_store, identity);
        crate::transport::ws::dispatch_roster_leave(orchestrator, &routing_id).await;
    }
    for identity in session.roster_snapshot() {
        let routing_id = super::startup_routing_id(routing_store, &identity);
        crate::transport::ws::dispatch_roster_join(orchestrator, &routing_id, &identity).await;
    }
}

pub(super) fn spawn_owner_park_loop(
    refresh: Arc<dyn RefreshDriver>,
    orchestrator: Arc<ProdOrchestrator>,
    ct: CancellationToken,
) {
    let owner_park = crate::orchestrator::OwnerParkBroadcaster::new(orchestrator, refresh.store());
    let mut events = refresh.store().subscribe();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = ct.cancelled() => return,
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

pub(super) async fn start_polling(orchestrator: &Arc<ProdOrchestrator>, has_refresh: bool) {
    orchestrator
        .dispatch(crate::orchestrator::OrchestratorAction::StartCiPolling)
        .await;
    if has_refresh {
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::StartIngestPolling)
            .await;
        orchestrator
            .dispatch(crate::orchestrator::OrchestratorAction::StartHydrationPolling)
            .await;
    }
}
