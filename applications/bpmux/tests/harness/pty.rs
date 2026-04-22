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

    /// Spawn bpmux with an extra env var set (on top of the usual test
    /// environment). Useful for features like `/tokens` (#179) that
    /// read paths from env vars so tests can redirect writes into a
    /// tempdir.
    pub fn start_with_env(name: &str, key: &str, value: &str) -> Self {
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, session_dir, &[(key, value)])
    }

    /// Spawn bpmux reusing an existing session base directory.
    /// Useful for testing restart scenarios where persisted state matters.
    pub fn start_with_session(name: &str, session_dir: tempfile::TempDir) -> Self {
        Self::start_with_session_and_env(name, session_dir, &[])
    }

    fn start_with_session_and_env(
        name: &str,
        session_dir: tempfile::TempDir,
        extra_env: &[(&str, &str)],
    ) -> Self {
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
        // Isolate the shared usage-counter file so tests don't read from
        // (or clobber) the developer's real ~/.bpmux/usage.json. Landed
        // with #230 when cross-session persistence started pulling that
        // file into App startup — a populated real file overflowed the
        // /tokens overlay in PTY tests.
        cmd.env("BPMUX_USAGE_PATH", session_dir.path().join("usage.json"));
        // Isolate the auth token file (#235) so each test gets its own
        // per-run secret. Without this, tests would either clobber the
        // developer's real ~/.bpmux/auth_token or inherit it — both
        // break reproducibility.
        cmd.env(
            "BPMUX_AUTH_TOKEN_PATH",
            session_dir.path().join("auth_token"),
        );
        for (k, v) in extra_env {
            cmd.env(*k, *v);
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

    /// POST JSON to a path on the spawned bpmux's REST surface with
    /// the per-run bearer token and loopback Host pin. Centralizes
    /// the auth boilerplate that would otherwise repeat across every
    /// simulate-endpoint test (#250). Returns the raw Response so
    /// callers can assert on status + body as needed. Rejection-path
    /// tests (wrong/missing bearer, spoofed host) intentionally
    /// bypass this helper and build their own request — their whole
    /// point is to deviate from the happy-path auth shape.
    pub async fn authed_post(&self, path: &str, body: serde_json::Value) -> reqwest::Response {
        let secret = self.simulate_secret();
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        reqwest::Client::new()
            .post(&url)
            .bearer_auth(&secret)
            .header(reqwest::header::HOST, format!("127.0.0.1:{}", self.port))
            .json(&body)
            .send()
            .await
            .expect("POST to bpmux should send")
    }

    /// Per-run auth token the spawned bpmux wrote at startup — tests
    /// that POST to /api/simulate/* need this in the Authorization
    /// header (#235).
    pub fn simulate_secret(&self) -> String {
        let dir = self
            .session_dir
            .as_ref()
            .expect("session dir taken before reading secret");
        let path = dir.path().join("auth_token");
        // The server generates the token before binding the TCP port.
        // `start_with_session_and_env` blocks until the TUI renders,
        // so the file should already exist; poll briefly in case of
        // timing jitter rather than panicking.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(s) = std::fs::read_to_string(&path) {
                let trimmed = s.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
            if Instant::now() >= deadline {
                panic!("auth_token never appeared at {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
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

    /// Poll until `cond` returns true or `timeout` expires. Returns
    /// `Ok(())` on success; on timeout returns an `Err` carrying the
    /// caller-supplied `what` description and the current TUI snapshot
    /// so the test failure message shows expected vs actual.
    ///
    /// Use this instead of `thread::sleep` when waiting for a UI state
    /// change — it returns as soon as the condition is met (fast path
    /// for conditions already true) and surfaces rich diagnostics when
    /// the condition never becomes true.
    pub fn wait_until<F>(&self, what: &str, timeout: Duration, mut cond: F) -> Result<(), String>
    where
        F: FnMut() -> bool,
    {
        let start = Instant::now();
        loop {
            if cond() {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(format!(
                    "timeout after {timeout:?} waiting for {what}; screen:\n{}",
                    self.tui()
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Poll until `target` is no longer visible in the TUI. Inverse of
    /// `wait_for_tui` — use it when a test waits for state to be
    /// cleared (e.g. a filter removed, an overlay closed).
    pub fn wait_for_tui_absent(&self, target: &str, timeout: Duration) -> Result<(), String> {
        self.wait_until(&format!("'{target}' to disappear"), timeout, || {
            !self.tui().contains(target)
        })
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
            "Enter" => "\r",
            "Alt-1" => "\x1b1",
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
