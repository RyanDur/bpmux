//! PTY helpers: running bpmux in a pseudo-terminal and observing its TUI.
#![allow(clippy::panic, clippy::expect_used)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use httpmock::{Method, MockServer};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::Value;

use super::mock_gh::{comment_json, issue_json};
use super::port::free_port;
use super::screen::Screen;

fn default_issue_json(number: u64, title: &str) -> Value {
    let body = format!(
        "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    issue_json(number, title, &[], Some(&body))
}

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
    /// Default httpmock pointed at the spawned bpmux's GH boundary
    /// (BPMUX_GH_BASE_URL). `None` when the caller supplied their own
    /// mock URL via `start_with_mock_gh` or extra env. Holding the
    /// MockServer here keeps it alive for the test's full duration.
    _default_mock_gh: Option<MockServer>,
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

    /// #599: spawn bpmux with the SERVER's `GitHubSourceAdapter` aimed
    /// at a test MockServer via `BPMUX_GH_BASE_URL`. Required for the
    /// cross_bridge_mutation_propagation suite post-#516 canonical-
    /// store unification — view_board + create_story now dispatch
    /// through the server's tracker, so setting `BPMUX_GH_BASE_URL`
    /// on the BRIDGE subprocess (the pre-#516 #482 pattern) became a
    /// no-op. `BPMUX_POLL_INTERVAL_SECS=1` so the server's ingest
    /// poll lands within the test's 2.5s wait window.
    pub fn start_with_mock_gh(mock_gh_base_url: &str) -> Self {
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(
            "human",
            session_dir,
            &[
                ("BPMUX_GH_BASE_URL", mock_gh_base_url),
                ("BPMUX_REPO", "acme/bpmux"),
                ("BPMUX_POLL_INTERVAL_SECS", "1"),
            ],
        )
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
        // Set BPMUX_ROOT to the actual repo root so view_roadmap can find
        // ROADMAP.md. CARGO_MANIFEST_DIR points at applications/bpmux/
        // post-workspace-refactor, and ROADMAP.md sits at the repo root
        // two parents up.
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root two parents up from CARGO_MANIFEST_DIR")
            .to_path_buf();
        cmd.env("BPMUX_ROOT", &workspace_root);
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
        cmd.env(
            "BPMUX_BOARD_STORE_PATH",
            session_dir.path().join("board-store.json"),
        );
        cmd.env(
            "BPMUX_BOARD_OUTBOX_PATH",
            session_dir.path().join("board-outbox.json"),
        );
        // #662: isolate the sessions-base directory so spawned bridge
        // subprocesses (which call `default_sessions_base()` for
        // bookmark + get_session_dir) read the per-test tempdir
        // instead of the developer's real `~/.bpmux/sessions/`.
        // Without this, parallel tests cross-contaminate via shared
        // bookmark / session-dir / current-symlink state. Mirrors
        // BPMUX_AUTH_TOKEN_PATH (#537) and BPMUX_USAGE_PATH (#230).
        cmd.env("BPMUX_SESSIONS_BASE", session_dir.path());
        // #724 slice 30: route the spawned bpmux's `log_diag` to the
        // canonical test-diag path instead of the developer's
        // `~/.bpmux/diag.log`. `session::diag::resolve_diag_path` walks
        // BPMUX_LOG → BPMUX_TEST_MODE → exe-has-deps → prod-default.
        // Subprocesses run from `target/debug/bpmux` (no `deps` segment)
        // so without an explicit signal they fall through to prod.
        // BPMUX_TEST_MODE *would* fix the diag route — but it ALSO
        // switches `run_server` to the InMemorySourceAdapter, breaking
        // every httpmock-based test that exercises the real
        // GitHubSourceAdapter. BPMUX_LOG is the orthogonal signal:
        // precedence-1, log-only, no other behavior change. Path matches
        // what `session::diag` resolves under exe-has-deps so the
        // diag_routing_test sees the same file. Mirrors
        // BPMUX_AUTH_TOKEN_PATH / BPMUX_USAGE_PATH / BPMUX_SESSIONS_BASE
        // — the default-isolation pattern. Pre-nextest cargo-test
        // serialized binaries so subprocess prod pollution never raced
        // `diag_routing_test`'s "prod untouched" assertion; nextest's
        // binary-level concurrency surfaced the gap (TRY 2 FAIL on the
        // slice-29 push).
        cmd.env(
            "BPMUX_LOG",
            std::env::temp_dir().join("bpmux-test-diag.log"),
        );
        // #724: when no caller-supplied mock URL is in extra_env,
        // point the spawned bpmux at a private empty-issues httpmock
        // by default. Without this, every `TestBpmux::start()` call
        // under the post-#534 pre-push gate (GH_TOKEN+BPMUX_REPO
        // mandatory) polls real github.com — production state floods
        // the TUI within seconds, breaking every "solo"-mode
        // assertion. Per Ryan's 2026-04-30 directive: only octocrab
        // gets test-doubled. Caller-side `start_with_mock_gh` still
        // wins because extra_env's BPMUX_GH_BASE_URL is applied AFTER
        // this default.
        let default_mock = if extra_env.iter().any(|(k, _)| *k == "BPMUX_GH_BASE_URL") {
            None
        } else {
            let server = MockServer::start();
            server.mock(|when, then| {
                when.method(Method::GET).path("/repos/acme/bpmux/issues");
                then.status(200)
                    .header("content-type", "application/json")
                    .body("[]");
            });
            // POST /issues returns a canned issue #1. httpmock's
            // first-match wins, so all create_story calls in a single
            // test get #1. Tests needing distinct numbers per
            // create_story must override via start_with_mock_gh.
            server.mock(|when, then| {
                when.method(Method::POST).path("/repos/acme/bpmux/issues");
                then.status(201)
                    .header("content-type", "application/json")
                    .body(default_issue_json(1, "test-target").to_string());
            });
            // Mutation endpoints for issues #1..=8 — gate-tools on
            // created or seeded targets exercise these. Includes a
            // generous range so tests that pass explicit numbers
            // (#100, #200, #300, #400, #500, #700, #777, #999...)
            // outside #1 still resolve. Without these, drain_pending
            // fails on the first label-add or comment-create.
            for n in 1..=8u64 {
                let issue_path = format!("/repos/acme/bpmux/issues/{n}");
                let labels_path = format!("/repos/acme/bpmux/issues/{n}/labels");
                let comments_path = format!("/repos/acme/bpmux/issues/{n}/comments");
                let labels_delete_re = format!(r"^/repos/acme/bpmux/issues/{n}/labels/.+$");
                let issue_get_path = issue_path.clone();
                server.mock(|when, then| {
                    when.method(Method::GET).path(issue_get_path);
                    then.status(200)
                        .header("content-type", "application/json")
                        .body(default_issue_json(n, "test-target").to_string());
                });
                server.mock(|when, then| {
                    when.method(Method::PATCH).path(issue_path);
                    then.status(200)
                        .header("content-type", "application/json")
                        .body(default_issue_json(n, "test-target").to_string());
                });
                server.mock(|when, then| {
                    when.method(Method::POST).path(labels_path);
                    then.status(200)
                        .header("content-type", "application/json")
                        .body("[]");
                });
                server.mock(|when, then| {
                    when.method(Method::POST).path(comments_path);
                    then.status(201)
                        .header("content-type", "application/json")
                        .body(comment_json("ok").to_string());
                });
                server.mock(|when, then| {
                    when.method(Method::DELETE)
                        .path_matches(regex::Regex::new(&labels_delete_re).unwrap());
                    then.status(200)
                        .header("content-type", "application/json")
                        .body("[]");
                });
            }
            cmd.env("BPMUX_GH_BASE_URL", format!("{}/", server.base_url()));
            cmd.env("BPMUX_REPO", "acme/bpmux");
            Some(server)
        };
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
            _default_mock_gh: default_mock,
        };
        // 15 s wait_for_tui: when 5+ TestBpmux subprocesses start
        // concurrently within a single test binary (story61, story65,
        // peer_count, sidebar_click etc. each have multiple #[test]
        // functions running in cargo's default num_cpus pool), the
        // "solo" render can stall past 5 s under load. The previous
        // 5 s budget was tight enough that ac4_inline_code_renders +
        // similar flaked one-in-N pre-push runs.
        assert!(
            bpmux.wait_for_tui("solo", Duration::from_secs(15)),
            "TUI should render"
        );
        bpmux.wait_for_http_ready();
        bpmux
    }

    /// Block until the bpmux HTTP listener is accepting requests.
    /// `wait_for_tui("solo")` confirms only the TUI rendered; the
    /// server's TCP listener bind happens AFTER
    /// `build_prod_orchestrator`'s ingest-with-retries (≈ 6 s
    /// worst-case under the post-#534 pre-push gate where GH_TOKEN +
    /// BPMUX_REPO are mandatory). Without this poll, every test that
    /// POSTs immediately after `start*` race-fails with
    /// connection-refused. 15 s timeout covers the worst-case retry
    /// schedule with headroom.
    ///
    /// Sync TCP-connect probe rather than reqwest — `#[tokio::test]`
    /// callers (share_message_test.rs etc.) are already inside a
    /// tokio runtime, so spinning up a nested runtime panics.
    fn wait_for_http_ready(&self) {
        let addr = format!("127.0.0.1:{}", self.port)
            .parse::<std::net::SocketAddr>()
            .expect("loopback addr parses");
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("bpmux HTTP server never became ready at {addr}");
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Path to the per-run auth_token file the spawned bpmux wrote at
    /// startup. Bridge subprocesses driving auth-gated affordances
    /// (simulate_*) need to read the SAME token file as the bpmux
    /// server; pass this path via `BPMUX_AUTH_TOKEN_PATH` env-var on
    /// `TestBridge::connect_with_env` so bridge's `load_auth_token()`
    /// reads from the test's tempdir instead of the developer's real
    /// `~/.bpmux/auth_token`. Closes the auth-token-cross-process gap
    /// (#487) for simulate-class agent-side E2E.
    pub fn auth_token_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading auth_token_path")
            .path()
            .join("auth_token")
    }

    /// #662: per-run sessions-base path. Bridge subprocesses pass this
    /// via `BPMUX_SESSIONS_BASE` so their `default_sessions_base()`
    /// resolves to the test's tempdir, not the developer's real
    /// `~/.bpmux/sessions/`. Closes the bridge-side cross-process
    /// session-state-leak under parallel-load.
    pub fn sessions_base_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading sessions_base_path")
            .path()
            .to_path_buf()
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
    ///
    /// Each call flushes immediately + parks briefly so crossterm's
    /// escape-sequence decoder in the child process has time to fully
    /// decode the sequence before any subsequent write arrives. Without
    /// the park, under concurrent test load the bare `\x1b` lead byte
    /// gets mis-parsed as a standalone Esc keypress while the trailing
    /// `[A` / `[<...M` / etc bytes echo as literal characters in the
    /// chat input field. Closes the flake-class previously documented
    /// at `tests/integration_main_modules/board_view.rs:93` and
    /// `story63_acceptance::ac1_unread_indicator_when_scrolled_up`.
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
        drop(w);
        std::thread::sleep(std::time::Duration::from_millis(20));
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
    ///
    /// Each event flushes immediately + parks briefly so crossterm's
    /// escape-sequence decoder has time to fully decode the SGR
    /// sequence before the next event arrives (see `type_key` doc for
    /// the underlying flake-class).
    pub fn mouse_scroll_up(&self, count: usize) {
        for _ in 0..count {
            let mut w = self.writer.lock().expect("writer lock");
            // SGR: \x1b[<64;col;rowM = scroll up (button 64)
            w.write_all(b"\x1b[<64;10;10M").expect("scroll up");
            w.flush().expect("flush");
            drop(w);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Send a mouse click at the given (col, row) position (1-based, SGR protocol).
    pub fn mouse_click(&self, col: u16, row: u16) {
        let mut w = self.writer.lock().expect("writer lock");
        // SGR: \x1b[<0;col;rowM = button press, \x1b[<0;col;rowm = button release
        let press = format!("\x1b[<0;{col};{row}M");
        let release = format!("\x1b[<0;{col};{row}m");
        w.write_all(press.as_bytes()).expect("mouse press");
        w.flush().expect("flush");
        drop(w);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut w = self.writer.lock().expect("writer lock");
        w.write_all(release.as_bytes()).expect("mouse release");
        w.flush().expect("flush");
        drop(w);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    /// Send mouse scroll-down events through the PTY (SGR mouse protocol).
    /// Same per-event flush+park as `mouse_scroll_up`.
    pub fn mouse_scroll_down(&self, count: usize) {
        for _ in 0..count {
            let mut w = self.writer.lock().expect("writer lock");
            // SGR: \x1b[<65;col;rowM = scroll down (button 65)
            w.write_all(b"\x1b[<65;10;10M").expect("scroll down");
            w.flush().expect("flush");
            drop(w);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl Drop for TestBpmux {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
