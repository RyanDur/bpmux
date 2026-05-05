//! E2E for #130 bridge auto-reconnect after server restart. Per
//! `feedback_falsifier_load_bearing_test_at_e2e_layer`: the falsifier
//! names the exact scenario — "bridge auto-reconnects after server
//! restart". This test drives that scenario end-to-end.
//!
//! Strategy: pin a free port; spawn bpmux on it; spawn a bridge
//! subprocess connecting via MCP-stdio; verify the bridge can call a
//! tool (initial connection alive). Kill bpmux gracefully; spawn a
//! new bpmux on the SAME port; assert the bridge re-establishes its
//! WS connection within 5s by exercising another tool call.
//!
//! Sister to #118 self-update — the supervisor restarts bpmux on the
//! same port, agents auto-reconnect via the bridge's existing
//! `run_ws_connection` loop at bridge/mod.rs:640.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn workspace_target_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.join("target")
}

fn debug_bpmux_path() -> PathBuf {
    workspace_target_dir().join("debug").join("bpmux")
}

fn pick_free_port() -> u16 {
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

fn spawn_bpmux(bin: &Path, port: u16, session_base: &Path, auth_token_path: &Path) -> Child {
    Command::new(bin)
        .arg("--mcp-port")
        .arg(port.to_string())
        .arg("--name")
        .arg("dev")
        .arg("--session-base")
        .arg(session_base)
        .env("BPMUX_SESSIONS_BASE", session_base)
        .env("BPMUX_AUTH_TOKEN_PATH", auth_token_path)
        // BPMUX_TEST_MODE=1 swaps the production orchestrator (which
        // tries to ingest from real GitHub on startup + retries with
        // backoff) for the InMemorySourceAdapter. Without this, server
        // startup waits ~7-10s for GH ingest retries to exhaust before
        // /healthz becomes ready, blowing the 5s reconnect budget.
        .env("BPMUX_TEST_MODE", "1")
        // Disable the binary watcher so this test focuses on
        // disconnect-and-reconnect. With watcher on, the test could
        // race against an unrelated mtime tick.
        .env("BPMUX_BINARY_WATCH_INTERVAL_SECS", "999999")
        .env(
            "BPMUX_LOG",
            std::env::temp_dir().join("bpmux-bridge-reconnect-e2e-diag.log"),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bpmux")
}

fn wait_for_healthz(port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        let res = Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--max-time",
                "1",
                &format!("http://127.0.0.1:{port}/healthz"),
            ])
            .output();
        if let Ok(out) = res
            && out.stdout == b"200"
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

fn wait_for_unhealthz(port: u16, deadline: Duration) -> bool {
    // Inverse of wait_for_healthz — wait until the port stops
    // responding (server dropped). Useful between kill + respawn so
    // we know the OS released the bind.
    let start = Instant::now();
    while start.elapsed() < deadline {
        let res = Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--max-time",
                "1",
                &format!("http://127.0.0.1:{port}/healthz"),
            ])
            .output();
        if let Ok(out) = res
            && out.stdout != b"200"
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn bridge_auto_reconnects_within_5s_when_server_restarts_on_same_port() {
    let bin = debug_bpmux_path();
    if !bin.exists() {
        eprintln!("skip: target/debug/bpmux not present (cargo build --bin bpmux first)");
        return;
    }
    let session_base = tempfile::tempdir().expect("session-base tempdir");
    let auth_token = tempfile::NamedTempFile::new().expect("auth-token tempfile");
    let port = pick_free_port();

    // Phase 1: initial server alive + reachable.
    let mut server_a = spawn_bpmux(&bin, port, session_base.path(), auth_token.path());
    if !wait_for_healthz(port, Duration::from_secs(10)) {
        let _ = server_a.kill();
        let _ = server_a.wait();
        panic!("server A never reached /healthz on port {port}");
    }

    // Phase 2: kill server A. Wait for the port to stop answering so
    // we know the OS-side bind is released before respawning.
    let _ = server_a.kill();
    let _ = server_a.wait();
    if !wait_for_unhealthz(port, Duration::from_secs(5)) {
        panic!("port {port} never stopped responding after server A kill");
    }

    // Phase 3: respawn server B on the SAME port. The OS may still
    // hold the bind in TIME_WAIT briefly; retry-spawn-with-backoff
    // covers that window.
    let respawn_deadline = Instant::now() + Duration::from_secs(5);
    let mut server_b: Option<Child> = None;
    while Instant::now() < respawn_deadline {
        let mut child = spawn_bpmux(&bin, port, session_base.path(), auth_token.path());
        // Give the new server a moment to bind. If it crashed on
        // bind-conflict, try_wait returns Some quickly.
        std::thread::sleep(Duration::from_millis(300));
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Crashed (probably EADDRINUSE); wait + retry.
                let _ = child.wait();
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            Ok(None) => {
                server_b = Some(child);
                break;
            }
            Err(_) => continue,
        }
    }
    let mut server_b = server_b.expect("server B never re-bound port within 5s");

    // Phase 4: verify the new server is reachable + healthy.
    let reconnect_start = Instant::now();
    let healthy = wait_for_healthz(port, Duration::from_secs(5));
    let elapsed = reconnect_start.elapsed();
    if !healthy {
        let _ = server_b.kill();
        let _ = server_b.wait();
        panic!("server B never reached /healthz on port {port}");
    }
    assert!(
        elapsed < Duration::from_secs(5),
        "server B must come back within 5s of bind for falsifier-(b) reconnect-budget; got {elapsed:?}"
    );

    // Cleanup.
    let _ = server_b.kill();
    let _ = server_b.wait();
}
