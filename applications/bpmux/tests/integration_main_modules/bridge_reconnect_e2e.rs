//! E2E for #130 bridge auto-reconnect after server restart. Per
//! `feedback_falsifier_load_bearing_test_at_e2e_layer`: the falsifier
//! names the BRIDGE-CLIENT behavior — bridge subprocess connected to
//! server A must auto-reconnect to server B (same port) within 5s.
//! This drives that exact cascade end-to-end:
//!
//! Phase 1: spawn bpmux server A on a pinned free port, connect a
//! TestBridge subprocess, call a tool to verify the WS handshake
//! completed.
//!
//! Phase 2: drop server A; wait for the port to stop responding so
//! we know the OS bind released.
//!
//! Phase 3: spawn server B on the SAME port, reusing the same
//! session_dir (matches #118 supervisor's restart-on-same-port
//! contract).
//!
//! Phase 4: call ANOTHER tool via the SAME bridge handle. Bridge's
//! existing `run_ws_connection` reconnect loop (bridge/mod.rs:640)
//! should re-establish the WS within its 3s sleep + identify-frame.
//! Tool call must succeed within a 5s budget per falsifier-(b).
//!
//! Sister to #118 self-update — server-restart-on-same-port is what
//! the supervisor does on exit-200; bridge reconnects to the new
//! instance via the existing loop without manual intervention.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use crate::harness::bridge::TestBridge;
use crate::harness::port::free_port;
use crate::harness::pty::TestBpmux;

fn wait_for_port_close(port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_millis(100),
        )
        .is_err()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn bridge_auto_reconnects_to_restarted_server_within_5s_on_same_port() {
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        // Phase 1: server A on a pinned free port + bridge connects.
        let port = free_port();
        let session_dir_a = tempfile::TempDir::new().expect("temp");
        let server_a = TestBpmux::start_with_session_and_port("dev", session_dir_a, port);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let bridge = TestBridge::connect(&server_a, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Sanity tool-call over the LIVE WS connection. set_name is
        // role-agnostic + cheap; success confirms the bridge ↔ server-A
        // handshake completed, identify-frame round-tripped, and the
        // WS is healthy.
        bridge
            .call_tool("set_name", serde_json::json!({ "name": "dev" }))
            .await
            .expect("baseline set_name on server A must succeed");

        // Phase 2: drop server A; wait for the port to release so the
        // OS doesn't hold us in TIME_WAIT.
        drop(server_a);
        assert!(
            wait_for_port_close(port, Duration::from_secs(5)),
            "server A's port {port} must release within 5s of drop"
        );

        // Phase 3: spawn server B on the SAME port. Reuses a
        // separate session_dir — content is irrelevant for this test;
        // we only care that the bridge reconnects to whatever's at
        // that port. Retry-spawn handles the brief TIME_WAIT race.
        let respawn_deadline = Instant::now() + Duration::from_secs(5);
        let mut server_b = None;
        let mut last_err: Option<String> = None;
        while Instant::now() < respawn_deadline {
            let session_dir_b = tempfile::TempDir::new().expect("temp");
            let result = std::panic::catch_unwind(|| {
                TestBpmux::start_with_session_and_port("dev", session_dir_b, port)
            });
            match result {
                Ok(b) => {
                    server_b = Some(b);
                    break;
                }
                Err(e) => {
                    last_err = Some(format!("{e:?}"));
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
        let _server_b = server_b
            .unwrap_or_else(|| panic!("server B never bound port {port} (last: {last_err:?})"));

        // Phase 4: bridge's reconnect loop sleeps up to 3s between
        // attempts, then re-handshakes. Within the 5s budget per
        // falsifier-(b), the SAME bridge handle should be able to
        // call another tool against the new server.
        let reconnect_start = Instant::now();
        let mut last_err: Option<String> = None;
        loop {
            if reconnect_start.elapsed() > Duration::from_secs(5) {
                panic!(
                    "bridge did not auto-reconnect to server B within 5s; last err: {last_err:?}"
                );
            }
            match bridge
                .call_tool("set_name", serde_json::json!({ "name": "dev" }))
                .await
            {
                Ok(_) => break,
                Err(e) => {
                    last_err = Some(e.to_string());
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
        let elapsed = reconnect_start.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "bridge auto-reconnect must complete within 5s for falsifier-(b); got {elapsed:?}"
        );
    });
}
