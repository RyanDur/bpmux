//! PTY helpers: running bpmux in a pseudo-terminal and observing its TUI.
#![allow(clippy::panic, clippy::expect_used)]

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
    session_dir: Option<tempfile::TempDir>,
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
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session(name, session_dir)
    }

    /// Spawn bpmux reusing an existing session base directory.
    /// Useful for testing restart scenarios where persisted state matters.
    pub fn start_with_session(name: &str, session_dir: tempfile::TempDir) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
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
        // Set BPMUX_ROOT to the actual repo root so /roadmap can find ROADMAP.md
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        cmd.env("BPMUX_ROOT", manifest_dir);

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
            session_dir: Some(session_dir),
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

    /// Take the session directory for reuse in a restart test.
    pub fn take_session_dir(&mut self) -> tempfile::TempDir {
        self.session_dir.take().expect("session dir already taken")
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

    /// Send a Ctrl+key combination (e.g. 'b' for Ctrl+B).
    pub fn type_ctrl(&self, ch: char) {
        let ctrl_byte = (ch as u8) & 0x1f;
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(&[ctrl_byte]).expect("type ctrl");
        w.flush().expect("flush");
    }

    /// Send a special key escape sequence (e.g. "Left", "Right", "Up", "Down").
    pub fn type_key(&self, key: &str) {
        let seq = match key {
            "Up" => "\x1b[A",
            "Down" => "\x1b[B",
            "Right" => "\x1b[C",
            "Left" => "\x1b[D",
            "Esc" => "\x1b",
            _ => panic!("unsupported key: {key}"),
        };
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(seq.as_bytes()).expect("type key");
        w.flush().expect("flush");
    }

    /// Simulate a bracketed paste: wraps text in paste-start/paste-end
    /// escape sequences so crossterm delivers it as `Event::Paste`.
    pub fn paste(&self, text: &str) {
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(b"\x1b[200~").expect("paste start");
        w.write_all(text.as_bytes()).expect("paste content");
        w.write_all(b"\x1b[201~").expect("paste end");
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

    /// Send a mouse click at the given (col, row) position (1-based, SGR protocol).
    pub fn mouse_click(&self, col: u16, row: u16) {
        let mut w = self.writer.lock().expect("writer lock");
        // SGR: \x1b[<0;col;rowM = button press, \x1b[<0;col;rowm = button release
        let press = format!("\x1b[<0;{col};{row}M");
        let release = format!("\x1b[<0;{col};{row}m");
        w.write_all(press.as_bytes()).expect("mouse press");
        w.write_all(release.as_bytes()).expect("mouse release");
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
