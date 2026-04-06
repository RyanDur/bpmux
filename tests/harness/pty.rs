//! PTY helpers: running bpmux in a pseudo-terminal and observing its TUI.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::port::free_port;
use super::screen::Screen;

/// A running bpmux instance in a PTY.
///
/// The drainer thread continuously reads the PTY and feeds a shared
/// `ScreenState` that tests can query via `wait_for_tui`. Without the
/// drainer the PTY buffer fills and bpmux blocks on writes, which is
/// what tests (and production) would experience with no terminal attached.
///
/// Uses a free port and a temporary session directory. Drop kills the
/// process; the drainer exits when the PTY closes.
pub struct TestBpmux {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _drainer: Option<std::thread::JoinHandle<()>>,
    _session_dir: tempfile::TempDir,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    screen: Arc<Mutex<Screen>>,
    port: u16,
}

impl TestBpmux {
    /// Spawn bpmux with the default identity ("human") and block until the
    /// TUI renders.
    pub fn start() -> Self {
        Self::start_named("human")
    }

    /// Spawn bpmux with a specific `--name` identity and block until the TUI
    /// renders.
    pub fn start_named(name: &str) -> Self {
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
        cmd.arg(name);
        cmd.arg("--mcp-port");
        cmd.arg(port.to_string());
        cmd.arg("--session-base");
        cmd.arg(&session_base);
        cmd.env("TERM", "xterm-256color");
        // Forward BPMUX_REPO so /board works in tests (no git repo in temp dir)
        if let Ok(repo) = std::env::var("BPMUX_REPO") {
            cmd.env("BPMUX_REPO", repo);
        }

        let child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().expect("clone reader");
        let writer = pair.master.take_writer().expect("take writer");

        let screen = Arc::new(Mutex::new(Screen::new()));
        let screen_feed = Arc::clone(&screen);
        let drainer = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                screen_feed.lock().expect("screen lock").feed(&buf[..n]);
            }
        });

        let bpmux = Self {
            child,
            _drainer: Some(drainer),
            _session_dir: session_dir,
            writer: Arc::new(Mutex::new(writer)),
            screen,
            port,
        };
        assert!(
            bpmux.wait_for_tui("solo", Duration::from_secs(5)),
            "TUI should render"
        );
        bpmux
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Poll the rendered TUI until it contains `target`. Returns true if
    /// found within `timeout`, false otherwise.
    pub fn wait_for_tui(&self, target: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.tui().contains(target) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Current TUI snapshot, rebuilt from accumulated bytes so callers
    /// never catch the screen mid-redraw.
    pub fn tui(&self) -> String {
        self.screen.lock().expect("screen lock").contents()
    }

    /// Type characters into the TUI without pressing Enter.
    pub fn type_chars(&self, text: &str) {
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(text.as_bytes()).expect("type text");
        w.flush().expect("flush");
    }

    /// Type text into the TUI and press Enter, as a user would.
    pub fn type_line(&self, text: &str) {
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(text.as_bytes()).expect("type text");
        w.write_all(b"\r").expect("press enter");
        w.flush().expect("flush");
    }

    /// Send mouse scroll-up events through the PTY (SGR mouse protocol).
    /// Each call sends `count` scroll-up events at position (col=10, row=10).
    pub fn mouse_scroll_up(&self, count: usize) {
        let mut w = self.writer.lock().expect("writer lock");
        for _ in 0..count {
            // SGR: \x1b[<64;col;rowM = scroll up (button 64)
            w.write_all(b"\x1b[<64;10;10M").expect("scroll up");
        }
        w.flush().expect("flush");
    }

    /// Send mouse scroll-down events through the PTY (SGR mouse protocol).
    pub fn mouse_scroll_down(&self, count: usize) {
        let mut w = self.writer.lock().expect("writer lock");
        for _ in 0..count {
            // SGR: \x1b[<65;col;rowM = scroll down (button 65)
            w.write_all(b"\x1b[<65;10;10M").expect("scroll down");
        }
        w.flush().expect("flush");
    }
}

impl Drop for TestBpmux {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
