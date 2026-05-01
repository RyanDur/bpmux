//! Bridge MCP client harness: spawn bridges, send messages, capture notifications.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rmcp::model::CustomNotification;
use rmcp::service::{NotificationContext, RunningService};
use rmcp::transport::child_process::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{ClientHandler, RoleClient, ServiceExt};

use super::pty::TestBpmux;

/// A bridge subprocess connected to a `TestBpmux`, exposing an MCP client.
///
/// Every bridge carries a `NotificationCapture`; tests that don't need
/// to observe notifications simply ignore `capture`.
pub struct TestBridge {
    client: RunningService<RoleClient, NotificationCapture>,
    pub capture: NotificationCapture,
}

/// Selects the bridge subprocess's source-of-truth.
///
/// `Fake` (default) passes `--fake-tracker` so the bridge uses
/// `InMemorySourceAdapter`. `RealAgainstMockGh` (#482) skips the
/// flag so the bridge builds a real `GitHubSourceAdapter` and reads
/// `BPMUX_GH_BASE_URL` from env to point at a test MockServer.
enum TrackerMode {
    Fake,
    RealAgainstMockGh,
}

impl TestBridge {
    /// Spawn a bridge subprocess with `--name` and connect an MCP client.
    pub async fn connect(bpmux: &TestBpmux, name: &str) -> Self {
        Self::connect_with_env(bpmux, name, &[]).await
    }

    /// Spawn a bridge subprocess with extra env vars. Useful for
    /// pointing the bridge's view_board at BPMUX_BOARD_FIXTURE instead
    /// of the live `gh` CLI in CI (where GH_TOKEN isn't wired).
    pub async fn connect_with_env(bpmux: &TestBpmux, name: &str, env: &[(&str, &str)]) -> Self {
        Self::connect_inner(bpmux, name, env, None, TrackerMode::Fake).await
    }

    pub async fn connect_with_board_fixture(
        bpmux: &TestBpmux,
        name: &str,
        fixture_path: &str,
    ) -> Self {
        Self::connect_inner(
            bpmux,
            name,
            &[],
            Some(fixture_path.to_string()),
            TrackerMode::Fake,
        )
        .await
    }

    /// Spawn a bridge subprocess WITHOUT `--fake-tracker`, pointed at
    /// a test-controlled GitHub MockServer via `BPMUX_GH_BASE_URL`
    /// (#482 cross-bridge substrate). All bridges launched against
    /// the same `mock_gh_base_url` share a single mock GitHub
    /// source-of-truth, enabling cross-bridge mutation-propagation
    /// tests where bridge A's writes are observable by bridge B's
    /// refresh path.
    pub async fn connect_with_mock_gh(
        bpmux: &TestBpmux,
        name: &str,
        mock_gh_base_url: &str,
    ) -> Self {
        // Fast poll so cross-bridge mutation propagation lands within
        // the test's tolerable wait window (~2s instead of the
        // production default 60s).
        let env: &[(&str, &str)] = &[
            ("BPMUX_GH_BASE_URL", mock_gh_base_url),
            ("BPMUX_REPO", "acme/bpmux"),
            ("BPMUX_POLL_INTERVAL_SECS", "1"),
        ];
        Self::connect_inner(bpmux, name, env, None, TrackerMode::RealAgainstMockGh).await
    }

    async fn connect_inner(
        bpmux: &TestBpmux,
        name: &str,
        env: &[(&str, &str)],
        board_fixture: Option<String>,
        tracker_mode: TrackerMode,
    ) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let port = bpmux.port();
        let name_owned = name.to_string();
        let mut env_owned: Vec<(String, String)> = env
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        // #537 default-propagate the per-test auth_token path so bridge
        // subprocess `load_auth_token()` reads the SAME file `TestBpmux`
        // wrote at startup. Without this, bridge falls back to the
        // developer's real `~/.bpmux/auth_token` (or 401s in CI), which
        // mismatches the per-test bpmux server's secret and every
        // auth-gated /api/rpc/* call returns 401. Existing explicit env
        // entries take precedence — caller can still override.
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_AUTH_TOKEN_PATH") {
            env_owned.push((
                "BPMUX_AUTH_TOKEN_PATH".to_string(),
                bpmux.auth_token_path().to_string_lossy().into_owned(),
            ));
        }
        // #662 default-propagate the per-test sessions-base so bridge
        // subprocess `default_sessions_base()` reads the same tempdir
        // the server wrote its session into. Without this, every
        // parallel test's bridges write/read the developer's real
        // `~/.bpmux/sessions/current` and cross-contaminate via
        // bookmark + session-dir state. Mirrors auth_token above.
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_SESSIONS_BASE") {
            env_owned.push((
                "BPMUX_SESSIONS_BASE".to_string(),
                bpmux.sessions_base_path().to_string_lossy().into_owned(),
            ));
        }
        // #724 slice 30: bridge subprocesses run from `target/debug/bpmux`
        // (no `deps` segment), so `session::diag` routes their `log_diag`
        // to `~/.bpmux/diag.log` unless an explicit signal overrides.
        // Use BPMUX_LOG (precedence-1, log-only) instead of
        // BPMUX_TEST_MODE — the latter also switches `run_server` to
        // the InMemorySourceAdapter, breaking every httpmock-based test.
        // Mirrors the server-spawn fix in `pty.rs`; together they close
        // the last shared-prod-file leak that flaked diag_routing_test
        // under nextest's binary-level concurrency.
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_LOG") {
            env_owned.push((
                "BPMUX_LOG".to_string(),
                std::env::temp_dir()
                    .join("bpmux-test-diag.log")
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        let board_fixture_owned = board_fixture;
        let process = TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
            cmd.arg("bridge")
                .arg("--port")
                .arg(port.to_string())
                .arg("--name")
                .arg(&name_owned);
            if matches!(tracker_mode, TrackerMode::Fake) {
                cmd.arg("--fake-tracker");
            }
            if let Some(ref path) = board_fixture_owned {
                cmd.arg("--board-fixture").arg(path);
            }
            for (k, v) in &env_owned {
                cmd.env(k, v);
            }
        }))
        .expect("spawn bridge");
        let capture = NotificationCapture::default();
        let client = capture
            .clone()
            .serve(process)
            .await
            .expect("MCP handshake with bridge");
        let bridge = Self { client, capture };
        bridge.wait_until_ws_ready(name).await;
        bridge
    }

    /// Probe set_name with the connect-time identity until it stops
    /// returning the "not connected to bpmux" error. set_name's
    /// handler calls ws_send, so a successful set_name proves the
    /// bridge→bpmux WebSocket handshake is complete. Re-using the
    /// caller's own name means this probe doesn't mutate the role the
    /// bridge identifies as (it just re-sets it to itself).
    async fn wait_until_ws_ready(&self, name: &str) {
        let _ = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let mut params = rmcp::model::CallToolRequestParams::new("set_name");
                params.arguments = serde_json::json!({ "name": name }).as_object().cloned();
                let result = self.client.peer().call_tool(params).await;
                if let Ok(ref tool_result) = result
                    && !tool_result.is_error.unwrap_or(false)
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;
    }

    /// Send a message through `send_message`, retrying until the bridge's
    /// WebSocket to bpmux is connected.
    pub async fn send(&self, text: &str) {
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let mut params = rmcp::model::CallToolRequestParams::new("send_message");
                params.arguments = serde_json::json!({"text": text}).as_object().cloned();
                let result = self
                    .client
                    .peer()
                    .call_tool(params)
                    .await
                    .expect("tool call");
                if !result.is_error.unwrap_or(false) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("bridge should become ready within timeout");
    }

    /// Poll `read_messages` until the returned content contains `substring`,
    /// or panic on timeout. Each poll consumes the unread buffer, so callers
    /// should only assert substrings that haven't been consumed already.
    pub async fn wait_for_message(&self, substring: &str, timeout: Duration) {
        let result = tokio::time::timeout(timeout, async {
            loop {
                let params = rmcp::model::CallToolRequestParams::new("read_messages");
                let result = self
                    .client
                    .peer()
                    .call_tool(params)
                    .await
                    .expect("read_messages");
                let text = format!("{result:?}");
                if text.contains(substring) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "message containing {substring:?} not read within {timeout:?}"
        );
    }

    /// Invoke an MCP tool and return the parsed JSON response. Use this
    /// when a test needs to assert on the shape/content of what an
    /// agent would see via MCP — story #199's agent-side assertion
    /// primitive. The return is the parsed `Value` of the first text
    /// content in `CallToolResult`; tests can then index fields
    /// (`result["number"]`, `result[0]["state"]`) directly.
    ///
    /// Returns `Err` with the tool's error text when the call returns
    /// `is_error: true`, so tests can assert on rejection messages
    /// (e.g. lane-enforcement checks).
    pub async fn call_tool(
        &self,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // The bridge-to-bpmux WebSocket handshake races the first
        // tool call under CI load. When ws_send fires before the
        // socket is up, the handler returns
        // "not connected to bpmux — reconnecting, try again in a
        // few seconds". Retry that specific error for up to 30s so
        // tests don't need to know the handshake timing.
        let mut params_template = rmcp::model::CallToolRequestParams::new(tool.to_string());
        params_template.arguments = args.as_object().cloned();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let mut params = rmcp::model::CallToolRequestParams::new(tool.to_string());
            params.arguments = params_template.arguments.clone();
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                self.client.peer().call_tool(params),
            )
            .await
            .map_err(|_| format!("call_tool({tool}) timed out"))?
            .map_err(|e| format!("call_tool({tool}) transport error: {e}"))?;
            let text = result
                .content
                .first()
                .and_then(|c| c.as_text().map(|t| t.text.clone()))
                .unwrap_or_default();
            if result.is_error.unwrap_or(false) {
                if text.contains("not connected to bpmux") && std::time::Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                return Err(text);
            }
            // Many tools return pretty-printed JSON; parse when possible,
            // but fall through to a string Value for plain-text responses
            // (e.g. status confirmations like "#200 accepted and closed").
            return Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)));
        }
    }

    /// Cancel the MCP client, bounded so cleanup can't hang.
    pub async fn close(self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.client.cancel()).await;
    }
}

/// Captures channel-notification payloads for assertions.
#[derive(Clone, Default)]
pub struct NotificationCapture {
    received: Arc<Mutex<Vec<String>>>,
}

impl NotificationCapture {
    /// Block until a captured notification content contains `substring`,
    /// or panic on timeout.
    pub async fn wait_for(&self, substring: &str, timeout: Duration) {
        let result = tokio::time::timeout(timeout, async {
            loop {
                let seen = self.received.lock().expect("capture lock").clone();
                if seen.iter().any(|s| s.contains(substring)) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "notification containing {substring:?} not received within {timeout:?}"
        );
    }

    pub fn received(&self) -> Vec<String> {
        self.received.lock().expect("capture lock").clone()
    }

    /// Test-only entry point that simulates a notification arrival,
    /// bypassing the rmcp `NotificationContext` plumbing.
    #[cfg(test)]
    fn push_for_test(&self, content: &str) {
        self.received
            .lock()
            .expect("capture lock")
            .push(content.to_string());
    }
}

impl ClientHandler for NotificationCapture {
    fn on_custom_notification(
        &self,
        notification: CustomNotification,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        let received = self.received.clone();
        async move {
            if notification.method == "notifications/claude/channel"
                && let Some(params) = notification.params
                && let Some(content) = params.get("content").and_then(|v| v.as_str())
            {
                received
                    .lock()
                    .expect("capture lock")
                    .push(content.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wait_for_returns_when_content_already_present() {
        let capture = NotificationCapture::default();
        capture.push_for_test("dev: hello there");
        capture
            .wait_for("hello there", Duration::from_millis(100))
            .await;
    }

    #[tokio::test]
    async fn wait_for_returns_when_content_arrives() {
        let capture = NotificationCapture::default();
        let writer = capture.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            writer.push_for_test("dev: arriving soon");
        });
        capture
            .wait_for("arriving soon", Duration::from_secs(1))
            .await;
    }

    #[tokio::test]
    #[should_panic(expected = "notification containing")]
    async fn wait_for_panics_on_timeout() {
        let capture = NotificationCapture::default();
        capture
            .wait_for("never arrives", Duration::from_millis(100))
            .await;
    }

    #[tokio::test]
    async fn received_returns_pushed_contents_in_order() {
        let capture = NotificationCapture::default();
        capture.push_for_test("first");
        capture.push_for_test("second");
        assert_eq!(capture.received(), vec!["first", "second"]);
    }
}
