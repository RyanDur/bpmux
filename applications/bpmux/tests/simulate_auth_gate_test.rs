//! End-to-end gate behavior for `/api/simulate/*` (#235).
//!
//! Unit tests in `src/transport/auth_tests.rs` cover the pure gate
//! logic (is_authorized / is_loopback_host). This file spins up the
//! real bpmux server through the PTY harness and confirms the gate
//! is actually wired into the axum router — unauthenticated requests
//! get 401, authenticated + loopback-host requests get 200.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Wait for the bpmux HTTP listener to bind. `wait_for_tui("solo")`
/// only confirms the TUI rendered — the server's listener bind happens
/// AFTER build_prod_orchestrator's ingest-with-retries (≈ 6 s
/// worst-case under the post-#534 pre-push gate).
async fn wait_for_http_ready(port: u16) {
    let url = format!("http://127.0.0.1:{port}/healthz");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let client = reqwest::Client::new();
    while tokio::time::Instant::now() < deadline {
        if client.get(&url).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("bpmux HTTP server never became ready at {url}");
}

#[test]
fn simulate_key_rejects_without_bearer_header() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let status = rt.block_on(async {
        wait_for_http_ready(port).await;
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1", "modifiers": ["alt"]}))
            .send()
            .await
            .expect("POST without auth")
            .status()
    });

    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "simulate/key without Authorization header must return 401"
    );
}

#[test]
fn simulate_key_rejects_wrong_bearer() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let status = rt.block_on(async {
        wait_for_http_ready(port).await;
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .bearer_auth("definitely-not-the-real-token")
            .json(&serde_json::json!({"key": "1"}))
            .send()
            .await
            .expect("POST with wrong auth")
            .status()
    });

    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
}

#[test]
fn simulate_click_rejects_without_bearer_header() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let status = rt.block_on(async {
        wait_for_http_ready(port).await;
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/api/simulate/click"))
            .json(&serde_json::json!({"x": 0_u16, "y": 0_u16}))
            .send()
            .await
            .expect("POST click without auth")
            .status()
    });

    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
}

#[test]
fn simulate_key_rejects_external_host_header_even_with_correct_secret() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    let secret = bpmux.simulate_secret();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let status = rt.block_on(async {
        wait_for_http_ready(port).await;
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .bearer_auth(&secret)
            // Simulates a DNS rebinding attack — attacker-controlled
            // public host rebinds to 127.0.0.1 at the network layer
            // but the Host header names the attacker's domain.
            .header(reqwest::header::HOST, "evil.com")
            .json(&serde_json::json!({"key": "1"}))
            .send()
            .await
            .expect("POST with spoofed host")
            .status()
    });

    assert_eq!(
        status,
        reqwest::StatusCode::UNAUTHORIZED,
        "even a correct secret must not grant access when Host names a non-loopback domain"
    );
}

#[test]
fn open_endpoints_unaffected_by_the_simulate_gate() {
    // `/api/messages` isn't in the gated sub-router — the middleware
    // should scope to simulate routes only. Read-only endpoints must
    // still answer without auth or we've over-applied the layer.
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let status = rt.block_on(async {
        wait_for_http_ready(port).await;
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/api/messages"))
            .send()
            .await
            .expect("GET messages")
            .status()
    });

    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "gate should only apply to /api/simulate/* — /api/messages stayed open pre-#235"
    );
}
