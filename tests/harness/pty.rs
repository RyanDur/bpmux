//! PTY helpers: reading TUI output and running bpmux in a pseudo-terminal.

use std::io::Read;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui_testlib::ScreenState;

use super::port::free_port;

/// Read from `reader` until the rendered screen contains `target` or the
/// timeout elapses. Returns the final screen contents and the raw bytes read.
pub fn read_until_contains(
    reader: &mut dyn Read,
    target: &str,
    timeout: Duration,
) -> (String, Vec<u8>) {
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
                let mut screen = ScreenState::new(80, 24);
                screen.feed(&all_bytes);
                if screen.contents().contains(target) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut screen = ScreenState::new(80, 24);
    screen.feed(&all_bytes);
    (screen.contents(), all_bytes)
}

/// A running bpmux instance in a PTY, with output drained in the background.
///
/// Uses a free port and a temporary session directory. Drops kill the
/// process; the drainer thread exits when the PTY closes.
pub struct TestBpmux {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _drainer: Option<std::thread::JoinHandle<()>>,
    _session_dir: tempfile::TempDir,
    port: u16,
}

impl TestBpmux {
    /// Spawn bpmux and block until the TUI renders.
    pub fn start() -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        let session_base = session_dir
            .path()
            .to_str()
            .expect("temp path is utf-8")
            .to_string();
        let port = free_port();

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
        cmd.arg("human");
        cmd.arg("--mcp-port");
        cmd.arg(port.to_string());
        cmd.arg("--session-base");
        cmd.arg(&session_base);
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().expect("clone reader");
        let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
        assert!(screen.contains("solo"), "TUI should render");

        let drainer = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
            }
        });

        Self {
            child,
            _drainer: Some(drainer),
            _session_dir: session_dir,
            port,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestBpmux {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
