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

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::Read;
use std::time::{Duration, Instant};

/// Read all available bytes from a reader with a timeout.
fn read_with_timeout(reader: &mut dyn Read, timeout: Duration) -> Vec<u8> {
    let mut all_bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let start = Instant::now();

    loop {
        if start.elapsed() >= timeout {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                all_bytes.extend_from_slice(&buf[..n]);
                if n < buf.len() {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
    all_bytes
}

/// Spawn bpmux in a PTY, read raw output, and return (raw_bytes, child).
/// The child must be killed by the caller.
fn spawn_and_read_bpmux(
    name: &str,
    read_timeout: Duration,
) -> (Vec<u8>, Box<dyn portable_pty::Child + Send + Sync>) {
    let bin = env!("CARGO_BIN_EXE_bpmux");

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
    cmd.env("TERM", "xterm-256color");

    let child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let raw = read_with_timeout(&mut reader, read_timeout);
    drop(reader);
    // Drop the master to signal EOF/hangup to the child
    drop(pair.master);

    (raw, child)
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

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let raw = read_with_timeout(&mut reader, Duration::from_secs(2));

    assert!(!raw.is_empty(), "Should have received bytes from echo");
    assert!(
        String::from_utf8_lossy(&raw).contains("HELLO_PTY_DIRECT"),
        "Raw bytes should contain echo output"
    );

    let _ = child.wait();
}

#[test]
fn bpmux_writes_to_pty() {
    let (raw, mut child) = spawn_and_read_bpmux("test-pty", Duration::from_secs(3));

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
    let (raw, mut child) = spawn_and_read_bpmux("uitest", Duration::from_secs(3));

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
