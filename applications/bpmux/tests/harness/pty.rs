#![allow(clippy::panic, clippy::expect_used)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use httpmock::{Method, MockServer};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::mock_gh::{
    MockIssueState, mock_issue_creation_allocates_sequential_numbers,
    mock_story_mutations_with_state,
};
use super::port::free_port;
use std::sync::Mutex as StdMutex;

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

    pub fn start_with_env(name: &str, key: &str, value: &str) -> Self {
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, session_dir, &[(key, value)])
    }

    pub fn start_with_envs(name: &str, extra_env: &[(&str, &str)]) -> Self {
        let session_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, session_dir, extra_env)
    }

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

    pub fn start_with_session(name: &str, session_dir: tempfile::TempDir) -> Self {
        Self::start_with_session_and_env(name, session_dir, &[])
    }

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
        let data_base = session_dir
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
        cmd.arg("--data-base");
        cmd.arg(&data_base);
        cmd.env("TERM", "xterm-256color");
        if let Ok(repo) = std::env::var("BPMUX_REPO") {
            cmd.env("BPMUX_REPO", repo);
        }
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root two parents up from CARGO_MANIFEST_DIR")
            .to_path_buf();
        cmd.env("BPMUX_ROOT", &workspace_root);
        cmd.env("BPMUX_PROJECT_ROOT", &workspace_root);
        cmd.env("BPMUX_USAGE_PATH", session_dir.path().join("usage.json"));
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
        cmd.env("BPMUX_DATA_BASE", session_dir.path());
        cmd.env("BPMUX_BRIEFINGS_DIR", session_dir.path().join("briefings"));
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
            mock_issue_creation_allocates_sequential_numbers(&server);
            for n in 1..=8u64 {
                let body = format!(
                    "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
                    env!("CARGO_MANIFEST_DIR")
                );
                let state = Arc::new(StdMutex::new(MockIssueState {
                    title: "test-target".to_string(),
                    body: Some(body),
                    labels: Vec::new(),
                    is_closed: false,
                }));
                mock_story_mutations_with_state(&server, n, state);
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

    pub fn usage_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading usage_path")
            .path()
            .join("usage.json")
    }

    pub fn sessions_base_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading sessions_base_path")
            .path()
            .to_path_buf()
    }

    pub fn bridge_cwd(&self, name: &str) -> std::path::PathBuf {
        let dir = self.sessions_base_path().join("bridge-cwd").join(name);
        std::fs::create_dir_all(&dir).expect("create isolated bridge cwd");
        dir
    }

    pub fn briefings_dir_path(&self) -> std::path::PathBuf {
        self.session_dir
            .as_ref()
            .expect("session dir taken before reading briefings_dir_path")
            .path()
            .join("briefings")
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
