#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::auth::{AuthState, gate, is_authorized, is_loopback_host};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, header};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use tower::ServiceExt;

fn headers(bearer: Option<&str>, host: Option<&str>) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(b) = bearer {
        h.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {b}")).unwrap(),
        );
    }
    if let Some(host) = host {
        h.insert(header::HOST, HeaderValue::from_str(host).unwrap());
    }
    h
}

#[test]
fn accepts_correct_bearer_and_loopback_host() {
    let h = headers(Some("secret-xyz"), Some("127.0.0.1:4117"));
    assert!(is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_missing_bearer() {
    let h = headers(None, Some("127.0.0.1:4117"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_wrong_bearer() {
    let h = headers(Some("wrong"), Some("127.0.0.1:4117"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_missing_host() {
    let h = headers(Some("secret-xyz"), None);
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_external_host_with_correct_bearer() {
    let h = headers(Some("secret-xyz"), Some("evil.com"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_suffix_trap_host() {
    let h = headers(Some("secret-xyz"), Some("127.0.0.1.evil.com"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_bearer_with_only_whitespace() {
    let mut h = HeaderMap::new();
    h.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Bearer    "),
    );
    h.insert(header::HOST, HeaderValue::from_static("127.0.0.1:4117"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn rejects_basic_auth_scheme() {
    let mut h = HeaderMap::new();
    h.insert(
        header::AUTHORIZATION,
        HeaderValue::from_static("Basic dXNlcjpwYXNz"),
    );
    h.insert(header::HOST, HeaderValue::from_static("127.0.0.1:4117"));
    assert!(!is_authorized(&h, "secret-xyz"));
}

#[test]
fn loopback_host_accepts_bare_127() {
    assert!(is_loopback_host("127.0.0.1"));
}

#[test]
fn loopback_host_accepts_127_with_port() {
    assert!(is_loopback_host("127.0.0.1:4117"));
}

#[test]
fn loopback_host_accepts_localhost() {
    assert!(is_loopback_host("localhost"));
    assert!(is_loopback_host("localhost:4117"));
}

#[test]
fn loopback_host_accepts_ipv6_loopback() {
    assert!(is_loopback_host("[::1]"));
    assert!(is_loopback_host("[::1]:4117"));
}

#[test]
fn loopback_host_rejects_external_dns() {
    assert!(!is_loopback_host("example.com"));
    assert!(!is_loopback_host("attacker.net:4117"));
}

#[test]
fn loopback_host_rejects_suffix_attacks() {
    assert!(!is_loopback_host("127.0.0.1.evil.com"));
    assert!(!is_loopback_host("localhost.evil.com"));
    assert!(!is_loopback_host("evil.127.0.0.1"));
}

#[test]
fn loopback_host_rejects_other_ipv4_addresses() {
    assert!(!is_loopback_host("0.0.0.0"));
    assert!(!is_loopback_host("192.168.1.1"));
    assert!(!is_loopback_host("10.0.0.1"));
}

fn gated_app() -> Router {
    Router::new()
        .route("/api/simulate", get(|| async { "ok" }))
        .layer(from_fn_with_state(
            AuthState::new("secret-xyz".to_string()),
            gate,
        ))
}

async fn reject_response(req: Request<Body>) -> (StatusCode, String, String) {
    let resp = gated_app().oneshot(req).await.expect("response");
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let body = String::from_utf8(bytes.to_vec()).expect("utf8 body");
    (status, content_type, body)
}

#[tokio::test]
async fn reject_body_is_byte_identical_across_all_three_causes() {
    let missing_token = Request::builder()
        .uri("/api/simulate")
        .header(header::HOST, "127.0.0.1:4117")
        .body(Body::empty())
        .expect("build missing-token request");
    let wrong_token = Request::builder()
        .uri("/api/simulate")
        .header(header::AUTHORIZATION, "Bearer wrong-token")
        .header(header::HOST, "127.0.0.1:4117")
        .body(Body::empty())
        .expect("build wrong-token request");
    let bad_host = Request::builder()
        .uri("/api/simulate")
        .header(header::AUTHORIZATION, "Bearer secret-xyz")
        .header(header::HOST, "evil.com")
        .body(Body::empty())
        .expect("build bad-host request");

    let missing = reject_response(missing_token).await;
    let wrong = reject_response(wrong_token).await;
    let host = reject_response(bad_host).await;

    assert_eq!(
        missing.0,
        StatusCode::UNAUTHORIZED,
        "reject must be 401, got {missing:?}"
    );
    assert_eq!(
        missing, wrong,
        "missing-token and wrong-token rejects must be byte-identical (status+content-type+body) — a difference is an auth-failure oracle"
    );
    assert_eq!(
        wrong, host,
        "wrong-token and bad-host rejects must be byte-identical (status+content-type+body) — a difference leaks which check failed"
    );
}
