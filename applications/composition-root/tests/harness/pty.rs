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

fn seeded_connections(
    base: &std::path::Path,
) -> integrations::data_gateway::GithubConnectionDataGateway {
    let data_dir = location::durable_data_dir(base).expect("durable data dir for seed");
    let db = Arc::new(database_support::DatabaseGateway::open(&data_dir).expect("open seed db"));
    integrations::data_gateway::GithubConnectionDataGateway::new(
        db,
        Arc::new(integrations::in_memory::InMemorySecretStore::default()),
    )
}

pub fn seed_github_connection(base: &std::path::Path, base_url: &str) {
    seeded_connections(base)
        .save(
            &integrations::github::connection::GithubConnection::WithBaseUrl {
                owner: "acme".to_string(),
                repo: "bpmux".to_string(),
                base_url: base_url.to_string(),
            },
        )
        .expect("seed github connection");
}

fn connection_already_seeded(base: &std::path::Path) -> bool {
    seeded_connections(base).resolve().ok().flatten().is_some()
}

pub struct TestBpmux {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _drainer: Option<std::thread::JoinHandle<()>>,
    data_base_dir: Option<tempfile::TempDir>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    port: u16,
    _default_mock_gh: Option<MockServer>,
}

fn describe_send_failure(error: &reqwest::Error, port: u16, path: &str) -> String {
    let mut chain = Vec::new();
    let mut source: Option<&dyn std::error::Error> = Some(error);
    while let Some(current) = source {
        chain.push(current.to_string());
        source = current.source();
    }
    format!(
        "POST {path} to 127.0.0.1:{port} never sent — the request failed at the CONNECTION layer, \
         so this says nothing about the handler. Read the chain to tell the causes apart: \
         'Connection refused' = the bpmux process is gone; 'Can't assign requested address' = the \
         CLIENT exhausted ephemeral ports (socket churn, not a server fault); 'Connection reset' = \
         the server died mid-request. is_connect={} is_timeout={} chain={chain:?}",
        error.is_connect(),
        error.is_timeout(),
    )
}

impl TestBpmux {
    pub fn start() -> Self {
        Self::start_named("human")
    }

    pub fn start_named(name: &str) -> Self {
        let data_base_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session(name, data_base_dir)
    }

    pub fn start_with_env(name: &str, key: &str, value: &str) -> Self {
        let data_base_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, data_base_dir, &[(key, value)])
    }

    pub fn start_with_envs(name: &str, extra_env: &[(&str, &str)]) -> Self {
        let data_base_dir = tempfile::TempDir::new().expect("temp dir");
        Self::start_with_session_and_env(name, data_base_dir, extra_env)
    }

    pub fn start_with_mock_gh(mock_gh_base_url: &str) -> Self {
        let data_base_dir = tempfile::TempDir::new().expect("temp dir");
        seed_github_connection(data_base_dir.path(), mock_gh_base_url);
        Self::start_with_session_and_env(
            "human",
            data_base_dir,
            &[("BPMUX_POLL_INTERVAL_SECS", "1")],
        )
    }

    pub fn start_with_session(name: &str, data_base_dir: tempfile::TempDir) -> Self {
        Self::start_with_session_and_env(name, data_base_dir, &[])
    }

    pub fn start_with_session_and_port(
        name: &str,
        data_base_dir: tempfile::TempDir,
        port: u16,
    ) -> Self {
        Self::start_with_session_and_env_and_port(name, data_base_dir, &[], Some(port))
    }

    fn start_with_session_and_env(
        name: &str,
        data_base_dir: tempfile::TempDir,
        extra_env: &[(&str, &str)],
    ) -> Self {
        Self::start_with_session_and_env_and_port(name, data_base_dir, extra_env, None)
    }

    fn start_with_session_and_env_and_port(
        name: &str,
        data_base_dir: tempfile::TempDir,
        extra_env: &[(&str, &str)],
        port_override: Option<u16>,
    ) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let data_base = data_base_dir
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
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root two parents up from CARGO_MANIFEST_DIR")
            .to_path_buf();
        cmd.env("BPMUX_ROOT", &workspace_root);
        cmd.env("BPMUX_PROJECT_ROOT", &workspace_root);
        cmd.env("BPMUX_USAGE_PATH", data_base_dir.path().join("usage.json"));
        cmd.env(
            "BPMUX_AUTH_TOKEN_PATH",
            data_base_dir.path().join("auth_token"),
        );
        cmd.env(
            "BPMUX_BOARD_STORE_PATH",
            data_base_dir.path().join("board-store.json"),
        );
        cmd.env(
            "BPMUX_PRIORITY_PATH",
            data_base_dir.path().join("priority.json"),
        );
        cmd.env("BPMUX_DATA_BASE", data_base_dir.path());
        cmd.env(
            "BPMUX_BRIEFINGS_DIR",
            data_base_dir.path().join("briefings"),
        );
        cmd.env("BPMUX_IDENTITY_DIR", data_base_dir.path().join("identity"));
        cmd.env(
            "BPMUX_LOG",
            std::env::temp_dir().join("bpmux-test-diag.log"),
        );
        cmd.env("BPMUX_TEST_SEED_CI_GREEN", "1");
        cmd.env("BPMUX_TEST_INMEMORY_SECRETS", "1");
        let default_mock = if connection_already_seeded(data_base_dir.path()) {
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
            seed_github_connection(data_base_dir.path(), &format!("{}/", server.base_url()));
            Some(server)
        };
        for (k, v) in extra_env {
            cmd.env(*k, *v);
        }

        let child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
        drop(pair.slave);

        let writer = pair.master.take_writer().expect("take writer");
        let drainer = spawn_output_drainer(pair.master.try_clone_reader().expect("clone reader"));

        let bpmux = Self {
            child,
            _drainer: Some(drainer),
            data_base_dir: Some(data_base_dir),
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
        self.data_base_dir
            .as_ref()
            .expect("data base taken before reading auth_token_path")
            .path()
            .join("auth_token")
    }

    pub fn usage_path(&self) -> std::path::PathBuf {
        self.data_base_dir
            .as_ref()
            .expect("data base taken before reading usage_path")
            .path()
            .join("usage.json")
    }

    pub fn data_base_path(&self) -> std::path::PathBuf {
        self.data_base_dir
            .as_ref()
            .expect("data base taken before reading data_base_path")
            .path()
            .to_path_buf()
    }

    pub fn bridge_cwd(&self, name: &str) -> std::path::PathBuf {
        let dir = self.data_base_path().join("bridge-cwd").join(name);
        std::fs::create_dir_all(&dir).expect("create isolated bridge cwd");
        dir
    }

    pub fn briefings_dir_path(&self) -> std::path::PathBuf {
        self.data_base_dir
            .as_ref()
            .expect("data base taken before reading briefings_dir_path")
            .path()
            .join("briefings")
    }

    pub fn identity_dir_path(&self) -> std::path::PathBuf {
        self.data_base_dir
            .as_ref()
            .expect("data base taken before reading identity_dir_path")
            .path()
            .join("identity")
    }

    pub async fn authed_post(&self, path: &str, body: serde_json::Value) -> reqwest::Response {
        let secret = self.simulate_secret();
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        match reqwest::Client::new()
            .post(&url)
            .bearer_auth(&secret)
            .header(reqwest::header::HOST, format!("127.0.0.1:{}", self.port))
            .json(&body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => panic!("{}", describe_send_failure(&e, self.port, path)),
        }
    }

    pub async fn authed_get(&self, path: &str) -> reqwest::Response {
        let secret = self.simulate_secret();
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        match reqwest::Client::new()
            .get(&url)
            .bearer_auth(&secret)
            .header(reqwest::header::HOST, format!("127.0.0.1:{}", self.port))
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => panic!("{}", describe_send_failure(&e, self.port, path)),
        }
    }

    pub fn simulate_secret(&self) -> String {
        let dir = self
            .data_base_dir
            .as_ref()
            .expect("data base taken before reading secret");
        let path = dir.path().join("auth_token");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(secret) = read_nonempty(&path) {
                return secret;
            }
            if Instant::now() >= deadline {
                panic!("auth_token never appeared at {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn take_data_base_dir(&mut self) -> tempfile::TempDir {
        self.data_base_dir
            .take()
            .expect("session dir already taken")
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

fn spawn_output_drainer(mut reader: Box<dyn Read + Send>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    })
}

fn read_nonempty(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    let trimmed = contents.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}
