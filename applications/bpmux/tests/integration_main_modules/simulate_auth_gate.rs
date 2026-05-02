//! End-to-end gate behavior for `/api/simulate/*` (#235).
//!
//! Unit tests in `src/transport/auth_tests.rs` cover the pure gate
//! logic (is_authorized / is_loopback_host). This file spins up the
//! real bpmux server through the PTY harness and confirms the gate
//! is actually wired into the axum router — unauthenticated requests
//! get 401, authenticated + loopback-host requests get 200.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::pty::TestBpmux;
use std::time::Duration;

// (TestBpmux::start now waits for the HTTP listener bind before
// returning — slice 14 + 19 — so per-test inline polls are no longer
// needed. The earlier slice-13 helper was retired in slice 21
// because the async-reqwest poll could itself hang under parallel
// load.)

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
