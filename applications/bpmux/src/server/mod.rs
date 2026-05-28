use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::{BoardStore, RefreshDriver};
use crate::session;
use crate::transport;
use crate::transport::auth::AuthState;

pub type TestRefreshHandle = Option<Arc<dyn RefreshDriver>>;

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
    screen_capture: Arc<std::sync::Mutex<String>>,
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
    for identity in session.roster_snapshot() {
        crate::transport::ws::dispatch_roster_join(&orchestrator, &identity).await;
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

    let gated_routes = axum::Router::new()
        .route("/api/board", axum::routing::get(transport::rest::get_board))
        .route(
            "/api/stories/{number}",
            axum::routing::get(transport::rest::get_story),
        )
        .route(
            "/api/stories/{number}/accept",
            axum::routing::post(transport::rest::post_accept_story),
        )
        .route(
            "/api/stories/{number}/reject",
            axum::routing::post(transport::rest::post_reject_story),
        )
        .route(
            "/api/stories/{number}/labels",
            axum::routing::post(transport::rest::post_story_labels),
        )
        .route(
            "/api/stories/{number}/comments",
            axum::routing::get(transport::rest::get_story_comments)
                .post(transport::rest::post_story_comment),
        )
        .route(
            "/api/stories/{number}/timeline",
            axum::routing::get(transport::rest::get_story_timeline),
        )
        .route(
            "/api/metrics",
            axum::routing::get(transport::rest::get_metrics),
        )
        .route("/api/ci", axum::routing::get(transport::rest::get_ci))
        .route(
            "/api/ci/log/{run_id}",
            axum::routing::get(transport::rest::get_ci_log),
        )
        .route(
            "/api/ci/rerun/{run_id}",
            axum::routing::post(transport::rest::post_ci_rerun),
        )
        .route(
            "/api/ci/jobs/{run_id}",
            axum::routing::get(transport::rest::get_ci_jobs),
        )
        .route(
            "/api/electron-capture",
            axum::routing::get(transport::rest::get_electron_capture),
        )
        .route(
            "/api/sync-status",
            axum::routing::get(transport::rest::get_sync_status),
        )
        .route(
            "/api/sync-status/force-sync",
            axum::routing::post(transport::rest::post_force_sync),
        )
        .route(
            "/api/sync-status/acknowledge",
            axum::routing::post(transport::rest::post_acknowledge_dropped),
        )
        .route(
            "/api/roster",
            axum::routing::get(transport::rest::get_roster),
        )
        .route(
            "/api/reactions",
            axum::routing::get(transport::rest::get_reactions).post(transport::rest::post_reaction),
        )
        .route(
            "/api/close_iteration",
            axum::routing::post(transport::rest::post_close_iteration),
        )
        .route(
            "/api/unfurl",
            axum::routing::get(transport::unfurl::get_unfurl).with_state(unfurl_cache),
        )
        .route(
            "/api/rpc/{tool}",
            axum::routing::post(transport::rpc_routes::post_rpc_tool),
        )
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            transport::auth::gate,
        ));

    let test_seed_routes = if transport::test_seed::seed_api_enabled() {
        axum::Router::new()
            .route(
                "/api/test/seed_messages",
                axum::routing::post(transport::test_seed::post_seed_messages),
            )
            .route(
                "/api/test/seed_reactions",
                axum::routing::post(transport::test_seed::post_seed_reactions),
            )
            .route(
                "/api/test/reset_messages",
                axum::routing::post(transport::test_seed::post_reset_messages),
            )
            .route(
                "/api/test/seed_roster",
                axum::routing::post(transport::test_seed::post_seed_roster),
            )
            .route(
                "/api/test/seed_stories",
                axum::routing::post(transport::test_seed::post_seed_stories),
            )
    } else {
        axum::Router::new()
    };

    let test_routes = if test_refresh.is_some() {
        axum::Router::new()
            .route(
                "/api/test/create_story",
                axum::routing::post(transport::test_routes::post_test_create_story),
            )
            .route(
                "/api/test/create_iteration",
                axum::routing::post(transport::test_routes::post_test_create_iteration),
            )
            .route(
                "/api/test/designate_current_iteration",
                axum::routing::post(transport::test_routes::post_test_designate_current_iteration),
            )
            .route(
                "/api/test/move_story",
                axum::routing::post(transport::test_routes::post_test_move_story),
            )
            .route(
                "/api/test/edit_story",
                axum::routing::post(transport::test_routes::post_test_edit_story),
            )
            .route(
                "/api/test/comment_on_story",
                axum::routing::post(transport::test_routes::post_test_comment_on_story),
            )
            .route(
                "/api/test/close_story",
                axum::routing::post(transport::test_routes::post_test_close_story),
            )
            .route(
                "/api/test/deliver_story",
                axum::routing::post(transport::test_routes::post_test_deliver_story),
            )
            .route(
                "/api/test/start_story",
                axum::routing::post(transport::test_routes::post_test_start_story),
            )
            .route(
                "/api/test/accept_story",
                axum::routing::post(transport::test_routes::post_test_accept_story),
            )
            .route(
                "/api/test/fail_desk_check",
                axum::routing::post(transport::test_routes::post_test_fail_desk_check),
            )
            .route(
                "/api/test/view_velocity",
                axum::routing::post(transport::test_routes::post_test_view_velocity),
            )
    } else {
        axum::Router::new()
    };

    let router = axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::get(transport::rest::get_messages).post(transport::rest::post_message),
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
            "/api/screen",
            axum::routing::get(transport::rest::get_screen),
        )
        .route(
            "/api/classifier-misfire",
            axum::routing::post(transport::rest::post_classifier_misfire),
        )
        .merge(gated_routes)
        .merge(test_routes)
        .merge(test_seed_routes)
        .with_state(session)
        .layer(axum::Extension(screen_capture))
        .layer(axum::Extension(tracker))
        .layer(axum::Extension(ci_state_store))
        .layer(axum::Extension(ci_log_store))
        .layer(axum::Extension(electron_capture_lock))
        .layer(axum::Extension(test_refresh))
        .layer(axum::Extension(orchestrator.clone()))
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

mod setup;
use setup::*;
