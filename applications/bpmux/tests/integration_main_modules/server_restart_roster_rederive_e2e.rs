#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use crate::harness::port::{free_port, wait_for_port_bindable};
use crate::harness::pty::TestBpmux;

async fn roster_identities(port: u16, secret: &str) -> Vec<String> {
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
    body["roster"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e["identity"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

async fn wait_for_roster_identity(
    port: u16,
    secret: &str,
    identity: &str,
    deadline: Duration,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if roster_identities(port, secret)
            .await
            .iter()
            .any(|i| i == identity)
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[test]
fn a_registered_agent_survives_a_server_restart_in_place_without_a_manual_reregister() {
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let port = free_port();
        let session_dir = tempfile::TempDir::new().expect("temp");
        let mut server_a = TestBpmux::start_with_session_and_port("ryan", session_dir, port);
        let secret = server_a.simulate_secret();

        let resp = server_a
            .authed_post(
                "/api/register",
                serde_json::json!({
                    "session_id": "sess-dev-restart",
                    "display": "dev",
                }),
            )
            .await;
        assert!(
            resp.status().is_success(),
            "register 'dev' on server A must succeed"
        );
        assert!(
            wait_for_roster_identity(port, &secret, "dev", Duration::from_secs(5)).await,
            "precondition: 'dev' is present in server A's roster after register"
        );

        let session_dir = server_a.take_session_dir();
        drop(server_a);
        assert!(
            wait_for_port_bindable(port, Duration::from_secs(10)),
            "server A's port {port} must become bindable after a restart-in-place drop"
        );

        let server_b = TestBpmux::start_with_session_and_port("ryan", session_dir, port);

        let reappeared =
            wait_for_roster_identity(port, &secret, "dev", Duration::from_secs(5)).await;
        let final_roster = roster_identities(port, &secret).await;
        drop(server_b);
        assert!(
            reappeared,
            "after a server-restart-in-place a still-registered agent 'dev' must be re-derived \
             from the PERSISTED RoutingStore into server B's roster — with no manual re-register. \
             The restart wipes the in-memory roster; the durable store survives a restart-in-place \
             and is the authoritative re-derive source (NOT the write-once roster_seed, which is \
             never persisted and re-joins nobody). Got roster: {final_roster:?}"
        );
    });
}
