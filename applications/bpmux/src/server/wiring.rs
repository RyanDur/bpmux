use std::sync::Arc;

use crate::room::{CommitmentStore, LivePtyRegistry};
use crate::routing::RoutingStore;
use crate::session;
use crate::store::Store;
use crate::transport;
use crate::transport::auth::AuthState;
use crate::transport::ci_log_store::CiLogStore;
use crate::transport::ci_state_store::CiStateStore;

use super::TestRefreshHandle;

pub(super) struct RouterDeps {
    pub auth_state: AuthState,
    pub unfurl_cache: transport::unfurl::UnfurlCacheHandle,
    pub session: Arc<session::Session>,
    pub tracker: transport::rest::TrackerHandle,
    pub ci_state_store: CiStateStore,
    pub ci_log_store: CiLogStore,
    pub electron_capture_lock: Arc<tokio::sync::Mutex<()>>,
    pub test_refresh: TestRefreshHandle,
    pub orchestrator: Arc<Store>,
    pub pty_registry: Arc<LivePtyRegistry>,
    pub routing_store: Arc<RoutingStore>,
    pub commitment_store: Arc<CommitmentStore>,
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
        commitment_store,
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
            "/api/wake-on-event",
            axum::routing::post(transport::rest::wake_on_event),
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
        .layer(axum::Extension(commitment_store))
        .layer(axum::Extension(port))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
}

pub(super) fn build_auth_state() -> anyhow::Result<AuthState> {
    let token_path = crate::auth::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot locate ~/.bpmux auth token path"))?;
    let token = crate::auth::ensure(&token_path).map_err(|e| {
        anyhow::anyhow!(
            "Unable to write auth token to {}: {e}",
            token_path.display()
        )
    })?;
    Ok(AuthState::new(token))
}
