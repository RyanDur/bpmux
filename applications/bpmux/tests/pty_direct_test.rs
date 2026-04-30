//! Direct PTY + termwiz test — bypasses ratatui-testlib to debug blank screen issue.
//!
//! Strategy: spawn bpmux in a real PTY via portable-pty, read raw bytes from the
//! master side, feed them into ratatui-testlib's ScreenState, then inspect screen.
//!
//! FINDINGS:
//! - bpmux writes ~2500 bytes of rich TUI output to the PTY (crossterm escape seqs)
//! - The data includes cursor positioning, box-drawing chars, colors, and text
//! - ratatui-testlib's ScreenState correctly parses raw bytes when fed directly
//! - The bug is in ratatui-testlib's update_state()/read() pipeline:
//!   TestTerminal::read() calls try_clone_reader() on every read, which on macOS
//!   may create competing readers that lose data

#![allow(clippy::unwrap_used, clippy::expect_used)]

use httpmock::{Method, MockServer};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::Value;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Read bytes from a reader in a background thread, collecting for `timeout`.
/// Uses a drainer thread so the blocking `read()` call doesn't prevent the
/// timeout from being respected (the async TUI only writes on events, not
/// continuously).
fn read_with_timeout(reader: Box<dyn Read + Send>, timeout: Duration) -> Vec<u8> {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let collected_feed = Arc::clone(&collected);

    let drainer = std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected_feed.lock().unwrap().extend_from_slice(&buf[..n]);
                }
                Err(_) => break,
            }
        }
    });

    std::thread::sleep(timeout);
    // Drainer may still be blocked in read(); that's fine — we take what we have.
    let result = collected.lock().unwrap().clone();
    drop(drainer); // Don't join — it may be blocked in read()
    result
}

/// Spawn bpmux in a PTY with an isolated session directory.
/// Returns (raw_bytes, child, _temp_dir, _mock_server). Keep _temp_dir
/// and _mock_server alive until done.
///
/// Per Ryan's 2026-04-30 directive (only octocrab gets test-doubled),
/// the spawned bpmux is pointed at an in-process httpmock returning an
/// empty issues list — so the TUI's solo-mode assertions don't fight
/// real GitHub state polled under GH_TOKEN+BPMUX_REPO.
fn spawn_and_read_bpmux(
    name: &str,
    read_timeout: Duration,
) -> (
    Vec<u8>,
    Box<dyn portable_pty::Child + Send + Sync>,
    tempfile::TempDir,
    MockServer,
) {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");

    let mock_server = MockServer::start();
    mock_server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![]).to_string());
    });
    let base_url = format!("{}/", mock_server.base_url());

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let mut cmd = CommandBuilder::new(bin);
    cmd.arg("--name");
    cmd.arg(name);
    cmd.arg("--session-base");
    cmd.arg(temp_dir.path().to_str().expect("temp path"));
    cmd.arg("--mcp-port");
    cmd.arg("0");
    cmd.env("TERM", "xterm-256color");
    cmd.env("BPMUX_GH_BASE_URL", &base_url);
    cmd.env("BPMUX_REPO", "acme/bpmux");

    let child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().expect("clone reader");
    let raw = read_with_timeout(reader, read_timeout);
    // Drop the master to signal EOF/hangup to the child
    drop(pair.master);

    (raw, child, temp_dir, mock_server)
}

#[test]
fn echo_through_pty() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("HELLO_PTY_DIRECT");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn echo");
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().expect("clone reader");
    let raw = read_with_timeout(reader, Duration::from_secs(2));

    assert!(!raw.is_empty(), "Should have received bytes from echo");
    assert!(
        String::from_utf8_lossy(&raw).contains("HELLO_PTY_DIRECT"),
        "Raw bytes should contain echo output"
    );

    let _ = child.wait();
}

#[test]
fn bpmux_writes_to_pty() {
    // 8 s read window: under the post-#534 pre-push gate (GH_TOKEN+
    // BPMUX_REPO mandatory), the bpmux server's startup polls real
    // GitHub before the TUI's first solo-mode render lands; 3 s was
    // tight enough that the status-bar "solo" text hadn't reached
    // the PTY yet.
    let (raw, mut child, _temp, _mock) = spawn_and_read_bpmux("test-pty", Duration::from_secs(8));

    eprintln!("bpmux wrote {} bytes to PTY", raw.len());

    // Feed into ratatui-testlib's ScreenState
    let mut screen = ratatui_testlib::ScreenState::new(80, 24);
    screen.feed(&raw);
    let contents = screen.contents();

    eprintln!("=== ScreenState parsed output ===");
    for (i, line) in contents.lines().enumerate() {
        if !line.trim().is_empty() {
            eprintln!("  [{i:2}] {line}");
        }
    }

    assert!(!raw.is_empty(), "bpmux should write output to the PTY");

    assert!(
        contents.contains("bpmux"),
        "ScreenState should parse the bpmux title. Contents:\n{contents}"
    );

    assert!(contents.contains("solo"), "Should show 'solo' status");

    eprintln!(
        "RESULT: bpmux writes {} bytes. ScreenState parses correctly.",
        raw.len()
    );
    eprintln!("The bug is in ratatui-testlib's read pipeline, not in parsing.");

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn bpmux_ui_elements_present() {
    // 8 s read window per bpmux_writes_to_pty rationale.
    let (raw, mut child, _temp, _mock) = spawn_and_read_bpmux("uitest", Duration::from_secs(8));

    let mut screen = ratatui_testlib::ScreenState::new(80, 24);
    screen.feed(&raw);
    let contents = screen.contents();

    // Title bar
    assert!(contents.contains("bpmux"), "Should have bpmux title");

    // Input prompt
    assert!(contents.contains(">"), "Should have input prompt '>'");

    // Status bar
    assert!(contents.contains("solo"), "Should show 'solo' status");

    // Box drawing chars
    assert!(
        contents.contains('\u{2502}'),
        "Should have vertical borders (|)"
    );
    assert!(contents.contains('\u{250c}'), "Should have top-left corner");
    assert!(
        contents.contains('\u{2500}'),
        "Should have horizontal borders (-)"
    );
    assert!(
        contents.contains('\u{2518}') || contents.contains('\u{2510}'),
        "Should have right-side corners"
    );

    let _ = child.kill();
    let _ = child.wait();
}
