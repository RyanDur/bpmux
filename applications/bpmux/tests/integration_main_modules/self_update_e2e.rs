//! E2E for #118 bpmux self-update on binary change. Per
//! `feedback_falsifier_load_bearing_test_at_e2e_layer`: falsifier-g
//! names the exact scenario — "binary-swap-and-restart cascade
//! end-to-end". This test drives that scenario:
//!
//! 1. Copy the bpmux binary to a temp path so we can mutate it
//!    without disturbing the parallel test suite (each cargo test
//!    binary's mtime would also be a fake-trigger if we touched the
//!    canonical target/debug/bpmux).
//! 2. Spawn the copy as a subprocess pointing at an isolated
//!    sessions-base + a free port.
//! 3. Wait for the server to start (probe /healthz).
//! 4. Touch the copy to change its mtime.
//! 5. Wait for the subprocess to exit. Assert the exit code is
//!    `RESTART_EXIT_CODE` (200) — the watcher saw the mtime change,
//!    fired the restart-signal, the server shut down gracefully,
//!    and main() exited via the signal-fired branch.
//!
//! State preservation (falsifier-d) is verified indirectly: bpmux's
//! session-dir + lessons + bookmarks + briefings + outbox are
//! disk-persisted and not touched by this test. A new spawn would
//! see the same on-disk state. Auto-reconnect (falsifier-c) is
//! sister-class #130; out of scope for this test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

const RESTART_EXIT_CODE: i32 = 200;

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

fn copy_binary_to_temp(src: &PathBuf, dest_dir: &tempfile::TempDir) -> PathBuf {
    let dest = dest_dir.path().join("bpmux-self-update-test");
    std::fs::copy(src, &dest).expect("copy bpmux binary to temp");
    let mut perms = std::fs::metadata(&dest).expect("metadata").permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&dest, perms).expect("set executable bit");
    dest
}

fn wait_for_healthz(port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        let res = std::process::Command::new("curl")
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

#[test]
fn binary_change_triggers_graceful_restart_with_exit_code_200() {
    let src = debug_bpmux_path();
    if !src.exists() {
        eprintln!("skip: target/debug/bpmux not present (cargo build --bin bpmux first)");
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let session_base = tempfile::tempdir().expect("session-base tempdir");
    let auth_token = tempfile::NamedTempFile::new().expect("auth-token tempfile");
    let copy = copy_binary_to_temp(&src, &tmp);
    let port = pick_free_port();
    let mut child = Command::new(&copy)
        .arg("--mcp-port")
        .arg(port.to_string())
        .arg("--name")
        .arg("dev")
        .env("BPMUX_SESSION_BASE", session_base.path())
        .env("BPMUX_DATA_BASE", session_base.path())
        .env("BPMUX_AUTH_TOKEN_PATH", auth_token.path())
        .env("BPMUX_BINARY_WATCH_INTERVAL_SECS", "1")
        .env("BPMUX_GH_BASE_URL", "http://127.0.0.1:1/")
        .env("BPMUX_REPO", "acme/bpmux")
        .env("GH_TOKEN", "fake-test-token")
        .env(
            "BPMUX_LOG",
            std::env::temp_dir().join("bpmux-self-update-e2e-diag.log"),
        )
        .env_remove("BPMUX_BINARY_WATCH_PATH")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn bpmux copy");

    let started = wait_for_healthz(port, Duration::from_secs(30));
    if !started {
        let _ = child.kill();
        let _ = child.wait();
        panic!("bpmux copy never reached /healthz on port {port}");
    }

    // Falsifier-(d) state-preservation drive: write a marker into the
    // session-base BEFORE triggering the restart-signal. After the
    // subprocess exits, assert the marker is still on disk + content
    // unchanged. This drives the load-bearing cascade per
    // feedback_falsifier_load_bearing_test_at_e2e_layer: state on
    // disk → restart-cycle fires → state still on disk + intact.
    let lessons_dir = session_base.path().join("lessons");
    std::fs::create_dir_all(&lessons_dir).expect("create lessons dir");
    let marker = lessons_dir.join("falsifier_d_state_preservation_marker.json");
    let marker_payload = b"falsifier-d: this content must survive the restart-cycle";
    std::fs::write(&marker, marker_payload).expect("write marker");

    // Sleep beyond filesystem mtime granularity (1s on most posix
    // FS; APFS sub-millisecond) so the touch produces a different
    // mtime that the watcher will observe.
    std::thread::sleep(Duration::from_millis(1100));

    // Linux blocks `write()` against a running executable with errno
    // ETXTBSY (kind ExecutableFileBusy) — text-file-busy lock — but
    // `rename()` over the same path is allowed because it swaps the
    // inode while the running process keeps its old inode mapped.
    // Write the new content to a sibling temp file + rename over the
    // path. Cross-platform: macOS doesn't enforce the lock so direct
    // write also works there, but rename is portable + matches the
    // way real binary swaps work in production (cargo build → mv).
    let mut bytes = std::fs::read(&copy).expect("read binary");
    bytes.push(0);
    let temp_path = copy.with_extension("new");
    std::fs::write(&temp_path, &bytes).expect("write new binary contents to temp");
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&temp_path)
        .expect("metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&temp_path, perms).expect("set exec bit on temp binary");
    std::fs::rename(&temp_path, &copy).expect("rename new binary over running path");

    // Wait for the subprocess to exit. The watcher polls every 1s
    // (BPMUX_BINARY_WATCH_INTERVAL_SECS=1); allow a generous window
    // for the graceful-shutdown cascade.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exit_status = None;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_status = Some(status);
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(200)),
            Err(e) => panic!("try_wait error: {e}"),
        }
    }
    let status = match exit_status {
        Some(s) => s,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("bpmux did not exit within 15s of binary mtime change");
        }
    };
    assert_eq!(
        status.code(),
        Some(RESTART_EXIT_CODE),
        "bpmux must exit with {RESTART_EXIT_CODE} (restart-signal) after binary-mtime change; got {:?}",
        status.code()
    );

    // Falsifier-(d) e2e assertion: marker we wrote into session-base
    // before the restart-cycle is still on disk + content intact.
    assert!(
        marker.exists(),
        "session-base disk content must persist across the restart-cycle per falsifier-(d); marker missing at {}",
        marker.display()
    );
    assert_eq!(
        std::fs::read(&marker).expect("read marker"),
        marker_payload,
        "marker content must be unchanged across the restart-cycle (no corruption / partial-write)"
    );
}
