#![allow(clippy::panic, clippy::expect_used)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use httpmock::{Method, MockServer};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::Value;

use super::mock_gh::{comment_json, issue_json};
use super::port::free_port;

fn default_issue_json(number: u64, title: &str) -> Value {
    let body = format!(
        "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    issue_json(number, title, &[], Some(&body))
}

pub struct TestBpmux {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _drainer: Option<std::thread::JoinHandle<()>>,
    session_dir: Option<tempfile::TempDir>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    port: u16,
    _default_mock_gh: Option<MockServer>,
}

impl TestBpmux {
    pub fn start() -> Self {
        Self::start_named("human")
    }

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

    /// Spawn bpmux with multiple extra env vars. Sister to `start_with_env`
    /// for callers that need to set more than one variable at startup
    /// (e.g., #1290 seed endpoints requiring both `BPMUX_TEST_SEED_API=1`
    /// and `BPMUX_STORAGE_BACKEND=memory`).
    pub fn start_with_envs(name: &str, extra_env: &[(&str, &str)]) -> Self {
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, session_dir, extra_env)
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

    /// Spawn bpmux on an explicit port, reusing an existing
    /// session_dir. Used by #130 bridge-reconnect e2e to verify
    /// server-restart-on-same-port → bridge auto-reconnect cascade.
    /// The caller is responsible for ensuring the port is free at
    /// spawn time (typically via `free_port()` immediately before
    /// calling this); test isolation otherwise mirrors `start_with_
    /// session_and_env`.
    pub fn start_with_session_and_port(
        name: &str,
        session_dir: tempfile::TempDir,
        port: u16,
    ) -> Self {
        Self::start_with_session_and_env_and_port(name, session_dir, &[], Some(port))
    }

    fn start_with_session_and_env(
        name: &str,
        session_dir: tempfile::TempDir,
        extra_env: &[(&str, &str)],
    ) -> Self {
        Self::start_with_session_and_env_and_port(name, session_dir, extra_env, None)
    }

    fn start_with_session_and_env_and_port(
        name: &str,
        session_dir: tempfile::TempDir,
        extra_env: &[(&str, &str)],
        port_override: Option<u16>,
    ) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let session_base = session_dir
            .path()
            .to_str()
            .expect("temp path is utf-8")
            .to_string();
        let port = port_override.unwrap_or_else(free_port);

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
        cmd.env(
            "BPMUX_PRIORITY_PATH",
            session_dir.path().join("priority.json"),
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
        cmd.env("BPMUX_TEST_SEED_CI_GREEN", "1");
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

        let drainer = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
            }
        });

        let bpmux = Self {
            child,
            _drainer: Some(drainer),
            session_dir: Some(session_dir),
            writer: Arc::new(Mutex::new(writer)),
            port,
            _default_mock_gh: default_mock,
        };
        bpmux.wait_for_http_ready();
        bpmux
    }

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

    pub fn auth_token_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading auth_token_path")
            .path()
            .join("auth_token")
    }

    pub fn sessions_base_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading sessions_base_path")
            .path()
            .to_path_buf()
    }

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

    pub fn simulate_secret(&self) -> String {
        let dir = self
            .session_dir
            .as_ref()
            .expect("session dir taken before reading secret");
        let path = dir.path().join("auth_token");
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

    pub fn take_session_dir(&mut self) -> tempfile::TempDir {
        self.session_dir.take().expect("session dir already taken")
    }

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
                return Err(format!("timeout after {timeout:?} waiting for {what}"));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for TestBpmux {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
