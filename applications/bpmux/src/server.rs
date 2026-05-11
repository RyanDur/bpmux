use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::session;
use crate::transport;
use crate::transport::auth::AuthState;

pub type TestRefreshHandle = Option<Arc<dyn RefreshDriver>>;

fn test_mode_enabled() -> bool {
    std::env::var("BPMUX_TEST_MODE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

async fn build_test_orchestrator(
    session: Arc<session::Session>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
) -> Arc<dyn RefreshDriver> {
    let mut corruption_detected = false;
    let store = match BoardStore::load_from_storage(session.storage.as_ref()) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            if matches!(e, crate::board::PersistError::Serde(_)) {
                corruption_detected = true;
            }
            crate::session::diag::log_diag(&format!(
                "server: BPMUX_TEST_MODE board-store disk-load failed ({e}); starting empty"
            ));
            Arc::new(BoardStore::new())
        }
    };
    let adapter = Arc::new(crate::board::adapters::in_memory::InMemorySourceAdapter::new());
    let orch_concrete = match SyncOrchestrator::with_storage(
        store.clone(),
        adapter.clone(),
        session.storage.clone(),
    ) {
        Ok(o) => o,
        Err(_) => SyncOrchestrator::new(store, adapter),
    };
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    if corruption_detected {
        orch.mark_recovering_from_corruption();
    }
    if let Err(e) = orch.refresh_all().await {
        crate::session::diag::log_diag(&format!(
            "server: BPMUX_TEST_MODE initial ingest skipped — {e}"
        ));
    }
    orch
}

async fn build_prod_orchestrator(
    session: Arc<session::Session>,
    adapter: Arc<crate::board::adapters::github::GitHubSourceAdapter>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
) -> Arc<dyn RefreshDriver> {
    let mut corruption_detected = false;
    let store = match BoardStore::load_from_storage(session.storage.as_ref()) {
        Ok(s) => {
            let count = s.all_stories().len();
            crate::session::diag::log_diag(&format!(
                "server: board-store hydrated via storage ({count} stories)"
            ));
            Arc::new(s)
        }
        Err(e) => {
            if matches!(e, crate::board::PersistError::Serde(_)) {
                corruption_detected = true;
            }
            crate::session::diag::log_diag(&format!(
                "server: board-store disk-load failed ({e}); starting empty"
            ));
            Arc::new(BoardStore::new())
        }
    };
    let orch_concrete = match SyncOrchestrator::with_storage(
        store.clone(),
        adapter.clone(),
        session.storage.clone(),
    ) {
        Ok(o) => {
            let pending_count = o.snapshot_status().pending_events;
            crate::session::diag::log_diag(&format!(
                "server: board-outbox hydrated via storage ({pending_count} pending events)"
            ));
            o
        }
        Err(e) => {
            crate::session::diag::log_diag(&format!(
                "server: board-outbox storage-load failed ({e}); starting with empty outbox"
            ));
            SyncOrchestrator::new(store.clone(), adapter.clone())
        }
    };
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    if corruption_detected {
        orch.mark_recovering_from_corruption();
        crate::session::diag::log_diag(
            "server: corruption-detected on board-store load; sync-status flagged recovering until first ingest succeeds",
        );
    }
    let backoffs = [0u64, 500, 1500, 4000];
    for (attempt, delay_ms) in backoffs.iter().enumerate() {
        if *delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
        }
        match orch.refresh_all().await {
            Ok(()) => {
                let count = orch.all_stories().len();
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest succeeded on attempt {} ({count} stories)",
                    attempt + 1
                ));
                if let Err(e) = store.persist_to_storage(session.storage.as_ref()) {
                    crate::session::diag::log_diag(&format!(
                        "server: post-ingest snapshot to storage failed: {e}"
                    ));
                }
                spawn_periodic_snapshot(store.clone(), session.storage.clone());
                return orch;
            }
            Err(e) if attempt + 1 < backoffs.len() => {
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest attempt {} failed: {e}; retrying with backoff",
                    attempt + 1
                ));
            }
            Err(e) => {
                let count = orch.all_stories().len();
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest exhausted retries; last error: {e}; BoardStore has {count} stories from disk-hydrate (read paths offline-capable)"
                ));
                spawn_periodic_snapshot(store.clone(), session.storage.clone());
            }
        }
    }
    orch
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
    screen_capture: Arc<std::sync::Mutex<String>>,
) -> anyhow::Result<()> {
    // Ensure the per-run auth token exists before binding the port.
    // Failing here is loud and blocks startup — simulate endpoints
    // can't be gated without a secret.
    let token_path = session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot locate ~/.bpmux auth token path"))?;
    let token = session::auth_token::ensure(&token_path).map_err(|e| {
        anyhow::anyhow!(
            "Unable to write auth token to {}: {e}",
            token_path.display()
        )
    })?;
    let auth_state = AuthState::new(token);

    // Board + story REST endpoints require a live tracker. Build once
    // at startup so each request doesn't re-auth with GitHub. None on
    // missing env — the handlers surface 503 "tracker not configured"
    // so the renderer can show a tracker-missing banner instead of a
    // blank board.
    let tracker: transport::rest::TrackerHandle =
        crate::board::adapters::github::GitHubSourceAdapter::from_env().map(std::sync::Arc::new);

    let (action_tx, action_rx) =
        tokio::sync::mpsc::unbounded_channel::<crate::orchestrator::OrchestratorAction>();

    let test_refresh: TestRefreshHandle = if test_mode_enabled() {
        eprintln!(
            "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
        );
        Some(build_test_orchestrator(session.clone(), Some(action_tx.clone())).await)
    } else if let Some(tracker_arc) = tracker.as_ref().cloned() {
        Some(build_prod_orchestrator(session.clone(), tracker_arc, Some(action_tx.clone())).await)
    } else {
        None
    };

    if let Some(refresh) = test_refresh.as_ref().cloned() {
        tokio::spawn(crate::bridge::run_drain_loop(
            refresh.clone(),
            session.clone(),
            crate::bridge::drain_interval_from_env(),
        ));
        // Boot heal sequence (#1268 → #1272 → #1283 → #1288 → #1271):
        // 1. #1268 evicts non-icebox stories from icebox_unified
        //    (canonical workflow_state truth-source)
        // 2. #1272 evicts non-ready stories from columns["ready"]
        //    legacy array (catches accept/close stragglers; dedups)
        // 3. #1283 restores markers from markers-store into
        //    icebox_unified when they're missing post-restart
        // 4. #1288 restores story-fold-positions adjacent to markers
        //    (must run AFTER #1283 — needs marker present; AFTER #1268
        //    — needs canonical icebox stories to filter evicted)
        // 5. #1271 evicts store-markers absent from icebox_unified
        //    (must run LAST so #1283/#1288 restores aren't evicted)
        let session_for_markers_heal = session.clone();
        tokio::spawn(async move {
            crate::auto_heal_icebox_at_boot(session_for_markers_heal.clone(), refresh.clone())
                .await;
            crate::auto_heal_ready_column_at_boot(
                session_for_markers_heal.clone(),
                refresh.clone(),
            )
            .await;
            crate::restore_marker_positions_to_icebox_unified_at_boot(
                session_for_markers_heal.clone(),
            )
            .await;
            crate::restore_story_fold_positions_to_icebox_unified_at_boot(
                session_for_markers_heal.clone(),
                refresh,
            )
            .await;
            crate::auto_heal_markers_store_orphans_at_boot(session_for_markers_heal).await;
        });
    }

    // #372: spawn the push-based CI state watcher. Polls GitHub at
    // its own cadence (independent of the 15s frontend poll). State
    // changes flow as typed StateDelta::Ci via the orchestrator's
    // CiSessionBroadcaster — renderer mirror.ci updates within WS
    // latency. Only spawns when we have a tracker — without one
    // `/api/ci` returns 503 and polling would just log errors.
    let ci_state_store = crate::transport::ci_state_store::CiStateStore::new();
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
    orchestrator.add_pins_listener(std::sync::Arc::new(
        crate::orchestrator::PinsSessionBroadcaster::new(session.clone()),
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

    // Unfurl cache (#343). One instance for the server lifetime so
    // all concurrent /api/unfurl requests share the memoized results.
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
            "/api/stories/{number}/reorder",
            axum::routing::post(transport::rest::post_reorder_story),
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
            "/api/pins",
            axum::routing::get(transport::rest::get_pins)
                .post(transport::rest::post_pin)
                .delete(transport::rest::delete_pin),
        )
        .route(
            "/api/reactions",
            axum::routing::get(transport::rest::get_reactions).post(transport::rest::post_reaction),
        )
        .route(
            "/api/iteration_markers",
            axum::routing::get(transport::rest::get_iteration_markers)
                .post(transport::rest::post_iteration_marker),
        )
        .route(
            "/api/close_iteration",
            axum::routing::post(transport::rest::post_close_iteration),
        )
        .route(
            "/api/iteration_markers/{id}/reorder",
            axum::routing::post(transport::rest::post_reorder_iteration_marker),
        )
        .route(
            "/api/add_stories_to_iteration",
            axum::routing::post(transport::rest::post_add_stories_to_iteration),
        )
        .route(
            "/api/remove_stories_from_iteration",
            axum::routing::post(transport::rest::post_remove_stories_from_iteration),
        )
        .route(
            "/api/iteration_markers/{id}/delete",
            axum::routing::post(transport::rest::post_delete_iteration_marker),
        )
        .route(
            "/api/unfurl",
            axum::routing::get(transport::unfurl::get_unfurl).with_state(unfurl_cache),
        )
        // #516 slices 1-3: generic RPC dispatch from team-bridges to
        // electron-server's canonical SyncOrchestrator. Path param names
        // the tool; body is the MCP args object. Mutation tools route
        // here; read tools migrate alongside the slice 4 cutover.
        .route(
            "/api/rpc/{tool}",
            axum::routing::post(transport::rpc_routes::post_rpc_tool),
        )
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            transport::auth::gate,
        ));

    // #1290 Part A: seed endpoints for Playwright e2e tests. Mounted
    // only when BPMUX_TEST_SEED_API=1; structural boundary, not runtime
    // conditional. Separate from BPMUX_TEST_MODE (which swaps to the
    // in-memory orchestrator) so launchWithMockGh tests can seed state
    // while keeping the prod orchestrator + mock-GH boundary.
    let test_seed_routes = if transport::test_seed::seed_api_enabled() {
        axum::Router::new()
            .route(
                "/api/test/seed_messages",
                axum::routing::post(transport::test_seed::post_seed_messages),
            )
            .route(
                "/api/test/seed_pins",
                axum::routing::post(transport::test_seed::post_seed_pins),
            )
            .route(
                "/api/test/seed_reactions",
                axum::routing::post(transport::test_seed::post_seed_reactions),
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
        // Liveness probe: tiny, no auth, no data. Electron's
        // spawn-or-adopt flow polls this to decide when the backend
        // is ready to accept requests. Kept separate from
        // `/api/messages` — which is auth-free by design but also
        // returns the full log and gets slower as the session
        // grows, eventually exceeding Electron's readiness timeout.
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
        // The Electron renderer loads from Vite's dev URL or file://
        // and fetches this backend at 127.0.0.1:4117 — a cross-origin
        // request that the browser preflight-checks. Without this
        // layer, every /api/* call fails with "Failed to fetch" while
        // the WebSocket (not subject to CORS) works fine. Server binds
        // to loopback only (see listener below), so permissive CORS
        // doesn't widen the network attack surface.
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        );
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
