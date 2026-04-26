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

async fn build_test_orchestrator(session: Arc<session::Session>) -> Arc<dyn RefreshDriver> {
    let _ = session;
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(crate::board::adapters::in_memory::InMemorySourceAdapter::new());
    let orch = Arc::new(SyncOrchestrator::new(store, adapter));
    if let Err(e) = orch.ingest_once().await {
        crate::session::diag::log_diag(&format!(
            "server: BPMUX_TEST_MODE initial ingest skipped — {e}"
        ));
    }
    orch
}

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
    screen_capture: Arc<std::sync::Mutex<String>>,
    inject_tx: tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
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
        crate::bridge::tracker::OctocrabStoryTracker::from_env().map(std::sync::Arc::new);

    let test_refresh: TestRefreshHandle = if test_mode_enabled() {
        eprintln!(
            "[bpmux server] BPMUX_TEST_MODE=1 — instantiating in-memory orchestrator for acceptance tests"
        );
        Some(build_test_orchestrator(session.clone()).await)
    } else {
        None
    };

    // #372: spawn the push-based CI state watcher. Polls GitHub at
    // its own cadence (independent of the 15s frontend poll), emits
    // CI_STATE_UPDATE_MARKER on detected conclusion change so the
    // renderer's CiGlyph can refetch /api/ci immediately instead of
    // waiting for its next local tick. Only spawns when we actually
    // have a tracker — without one `/api/ci` returns 503 and polling
    // would just log errors.
    if let Some(tracker_arc) = tracker.as_ref().cloned() {
        crate::transport::ci_watcher::spawn(session.clone(), tracker_arc, ct.clone());
    }

    // Unfurl cache (#343). One instance for the server lifetime so
    // all concurrent /api/unfurl requests share the memoized results.
    let unfurl_cache: transport::unfurl::UnfurlCacheHandle =
        std::sync::Arc::new(transport::unfurl::UnfurlCache::new());

    // Auth-gated sub-router: simulate_* (existing) + the owner-gate
    // board/story mutations (new). All routes here sit behind the
    // bearer+loopback middleware — a single token check covers both
    // the TUI-injection surface and the issue-mutation surface.
    let gated_routes = axum::Router::new()
        .route(
            "/api/simulate/click",
            axum::routing::post(transport::rest::simulate_click),
        )
        .route(
            "/api/simulate/key",
            axum::routing::post(transport::rest::simulate_key),
        )
        .route(
            "/api/simulate/type",
            axum::routing::post(transport::rest::simulate_type),
        )
        .route(
            "/api/simulate/input_clear",
            axum::routing::post(transport::rest::simulate_input_clear),
        )
        .route(
            "/api/simulate/input_submit",
            axum::routing::post(transport::rest::simulate_input_submit),
        )
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
            "/api/unfurl",
            axum::routing::get(transport::unfurl::get_unfurl).with_state(unfurl_cache),
        )
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            transport::auth::gate,
        ));

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
        .with_state(session)
        .layer(axum::Extension(screen_capture))
        .layer(axum::Extension(inject_tx))
        .layer(axum::Extension(tracker))
        .layer(axum::Extension(test_refresh))
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
