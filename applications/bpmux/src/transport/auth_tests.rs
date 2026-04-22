#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::auth::{is_authorized, is_loopback_host};
use axum::http::{HeaderMap, HeaderValue, header};

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
    // Classic suffix match attack: attacker controls evil.com, hopes
    // naive "starts_with 127.0.0.1" check lets them in.
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
    // Header exists but isn't Bearer — should fail to parse.
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
