#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::harness::pty::TestBpmux;

async fn roster_presence_for_role(port: u16, secret: &str, role: &str) -> Option<String> {
    let url = format!("http://127.0.0.1:{port}/api/roster");
    let body: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .bearer_auth(secret)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
        .send()
        .await
        .expect("GET /api/roster")
        .json()
        .await
        .expect("roster json");
    body["roster"].as_array().and_then(|arr| {
        arr.iter()
            .find(|entry| entry["role"].as_str() == Some(role))
            .and_then(|entry| entry["presence"].as_str().map(str::to_string))
    })
}

async fn wait_for_role_presence(
    port: u16,
    secret: &str,
    role: &str,
    want: &str,
    deadline: Duration,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if roster_presence_for_role(port, secret, role)
            .await
            .as_deref()
            == Some(want)
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

async fn run_signal(
    bin: &str,
    port: u16,
    token_path: &std::path::Path,
    role: &str,
    state: &str,
) -> (bool, String) {
    let output = tokio::process::Command::new(bin)
        .arg("signal")
        .arg(state)
        .env("BPMUX_PORT", port.to_string())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_AUTH_TOKEN_PATH", token_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("run bpmux signal");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), combined)
}

#[tokio::test]
async fn an_agent_pushes_its_state_via_bpmux_signal_and_the_roster_dot_reflects_it() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let secret = bpmux.simulate_secret();
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let token_path = bpmux.auth_token_path();

    let registered = bpmux
        .authed_post(
            "/api/register",
            serde_json::json!({ "session_id": "sess-dev-signal", "display": "dev" }),
        )
        .await;
    assert!(
        registered.status().is_success(),
        "precondition: dev registers, so the roster holds a member to resolve and colour"
    );

    let (ok_busy, out_busy) = run_signal(bin, port, &token_path, "dev", "busy").await;
    assert!(
        ok_busy,
        "`bpmux signal busy` must succeed end-to-end: the REAL CLI -> gated /api/presence -> roster Presence. output: {out_busy}"
    );
    assert!(
        wait_for_role_presence(port, &secret, "dev", "Busy", Duration::from_secs(5)).await,
        "after `bpmux signal busy`, dev's roster presence (the collaboration-dot's source) reads Busy — the agent's OWN pushed state, with no terminal-quiet inference in play (it was retired in this slice)"
    );

    let (ok_help, out_help) = run_signal(bin, port, &token_path, "dev", "help").await;
    assert!(
        ok_help,
        "`bpmux signal help` must succeed end-to-end. output: {out_help}"
    );
    assert!(
        wait_for_role_presence(port, &secret, "dev", "Help", Duration::from_secs(5)).await,
        "needs-help is reachable BY THE AGENT through the signal verb — closing the Help-never-set gap (owner-park previously set only Blocked)"
    );
}

#[tokio::test]
async fn a_presence_push_without_a_bearer_token_is_rejected_by_the_gate() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    let url = format!("http://127.0.0.1:{port}/api/presence");
    let resp = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::HOST, format!("127.0.0.1:{port}"))
        .json(&serde_json::json!({ "state": "busy" }))
        .send()
        .await
        .expect("POST /api/presence without a bearer token");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "presence is an authenticated agent action: a push with no Bearer token is rejected by the gate (401), never silently accepted"
    );
}
