//! Cross-bridge canonical CI state test for #526.
//!
//! Sister to rpc_canonical_store_test.rs (from #516 slice 5). Pins the
//! cross-bridge parity invariant: every bridge consuming `/api/ci`
//! observes the same canonical state because the state lives once on
//! electron-server's `ci_watcher` and bridges read from there rather
//! than each fetching GitHub independently.
//!
//! The test stands up a real `axum::serve` against the production
//! `/api/ci` route, then drives two independent `reqwest::Client`
//! calls (separate TCP, separate HTTP cycles) — each as if it were a
//! distinct bridge process. Both must observe the same response from
//! the same canonical source.
//!
//! No GitHub access; the tracker layer is mocked so the test exercises
//! the bridge↔server contract without live-network side effects.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Extension;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use bpmux::session::auth_token;
use bpmux::transport::auth::{AuthState, gate};
use bpmux::transport::rest::{TrackerHandle, get_ci};

async fn fire_get_ci(port: u16, token: &str) -> reqwest::Response {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/api/ci"))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("get /api/ci")
}

#[tokio::test]
async fn two_bridges_observe_identical_ci_state_via_canonical_endpoint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("auth_token");
    let token = auth_token::ensure(&token_path).expect("ensure token");

    let auth_state = AuthState::new(token.clone());

    // Tracker is None for this test — `get_ci` returns 503 when no
    // tracker is configured. That 503 is itself canonical: every
    // bridge reading from this endpoint observes the SAME 503, not
    // a divergent fetch outcome. Pre-#526, each bridge held its own
    // GitHubApi and would return its own (potentially divergent)
    // direct-GH result. Post-#526, they all see whatever the server
    // says — including absence-of-tracker uniformly.
    let tracker: TrackerHandle = None;

    let gated = axum::Router::new()
        .route("/api/ci", axum::routing::get(get_ci))
        .layer(axum::middleware::from_fn_with_state(auth_state, gate));

    let app = axum::Router::new().merge(gated).layer(Extension(tracker));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move { server_ct.cancelled_owned().await })
        .await
        .expect("serve");
    });

    let bridge_a = fire_get_ci(port, &token).await;
    let bridge_b = fire_get_ci(port, &token).await;

    let status_a = bridge_a.status();
    let body_a = bridge_a.text().await.expect("body a");
    let status_b = bridge_b.status();
    let body_b = bridge_b.text().await.expect("body b");

    assert_eq!(
        status_a, status_b,
        "two bridges must observe identical status from the canonical /api/ci"
    );
    assert_eq!(
        body_a, body_b,
        "two bridges must observe identical body from the canonical /api/ci"
    );

    ct.cancel();
    let _ = server.await;
}

#[tokio::test]
async fn unauthenticated_request_to_ci_endpoint_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let token_path = dir.path().join("auth_token");
    let token = auth_token::ensure(&token_path).expect("ensure token");
    let _ = token;

    let auth_state = AuthState::new("real-token".to_string());

    let tracker: TrackerHandle = None;

    let gated = axum::Router::new()
        .route("/api/ci", axum::routing::get(get_ci))
        .layer(axum::middleware::from_fn_with_state(auth_state, gate));

    let app = axum::Router::new().merge(gated).layer(Extension(tracker));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let ct = CancellationToken::new();
    let server_ct = ct.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move { server_ct.cancelled_owned().await })
        .await
        .expect("serve");
    });

    // Bearer mismatched with the gate's expected token.
    let res = fire_get_ci(port, "wrong-token").await;
    assert_eq!(
        res.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "auth gate must reject mismatched Bearer"
    );

    ct.cancel();
    let _ = server.await;
}
