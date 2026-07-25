#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Extension;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use auth as auth_token;
use transport::auth::{AuthState, gate};
use transport::rest::{TrackerHandle, get_ci};

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

    let res = fire_get_ci(port, "wrong-token").await;
    assert_eq!(
        res.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "auth gate must reject mismatched Bearer"
    );

    ct.cancel();
    let _ = server.await;
}
