use std::time::Duration;

use rmcp::service::RunningService;
use rmcp::transport::child_process::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{ClientHandler, RoleClient, ServiceExt};

use super::pty::TestBpmux;

#[derive(Clone, Default)]
struct NoopHandler;

impl ClientHandler for NoopHandler {}

pub struct TestBridge {
    client: RunningService<RoleClient, NoopHandler>,
}

enum TrackerMode {
    Fake,
    RealAgainstMockGh,
}

impl TestBridge {
    pub async fn connect(bpmux: &TestBpmux, name: &str) -> Self {
        Self::connect_with_env(bpmux, name, &[]).await
    }

    pub async fn connect_with_env(bpmux: &TestBpmux, name: &str, env: &[(&str, &str)]) -> Self {
        Self::connect_inner(bpmux, name, env, TrackerMode::Fake).await
    }

    pub async fn connect_with_mock_gh(
        bpmux: &TestBpmux,
        name: &str,
        mock_gh_base_url: &str,
    ) -> Self {
        let env: &[(&str, &str)] = &[
            ("BPMUX_GH_BASE_URL", mock_gh_base_url),
            ("BPMUX_REPO", "acme/bpmux"),
            ("BPMUX_POLL_INTERVAL_SECS", "1"),
        ];
        Self::connect_inner(bpmux, name, env, TrackerMode::RealAgainstMockGh).await
    }

    pub async fn connect_from_spawned_config(
        bpmux: &TestBpmux,
        program: &str,
        config_args: &[String],
        wait_name: &str,
    ) -> Self {
        let env_owned: Vec<(String, String)> = vec![
            (
                "BPMUX_AUTH_TOKEN_PATH".to_string(),
                bpmux.auth_token_path().to_string_lossy().into_owned(),
            ),
            (
                "BPMUX_DATA_BASE".to_string(),
                bpmux.sessions_base_path().to_string_lossy().into_owned(),
            ),
            (
                "BPMUX_USAGE_PATH".to_string(),
                bpmux.usage_path().to_string_lossy().into_owned(),
            ),
            (
                "BPMUX_LOG".to_string(),
                std::env::temp_dir()
                    .join("bpmux-test-diag.log")
                    .to_string_lossy()
                    .into_owned(),
            ),
            (
                "BPMUX_BRIEFINGS_DIR".to_string(),
                bpmux.briefings_dir_path().to_string_lossy().into_owned(),
            ),
        ];
        let args: Vec<String> = config_args.to_vec();
        let process =
            TokioChildProcess::new(tokio::process::Command::new(program).configure(|cmd| {
                for a in &args {
                    cmd.arg(a);
                }
                cmd.arg("--fake-tracker");
                for (k, v) in &env_owned {
                    cmd.env(k, v);
                }
            }))
            .expect("spawn bridge from the spawn-produced config");
        let client = NoopHandler
            .serve(process)
            .await
            .expect("MCP handshake with bridge from produced config");
        let bridge = Self { client };
        bridge.wait_until_ws_ready(wait_name).await;
        bridge
    }

    async fn connect_inner(
        bpmux: &TestBpmux,
        name: &str,
        env: &[(&str, &str)],
        tracker_mode: TrackerMode,
    ) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let port = bpmux.port();
        let name_owned = name.to_string();
        let mut env_owned: Vec<(String, String)> = env
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_AUTH_TOKEN_PATH") {
            env_owned.push((
                "BPMUX_AUTH_TOKEN_PATH".to_string(),
                bpmux.auth_token_path().to_string_lossy().into_owned(),
            ));
        }
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_DATA_BASE") {
            env_owned.push((
                "BPMUX_DATA_BASE".to_string(),
                bpmux.sessions_base_path().to_string_lossy().into_owned(),
            ));
        }
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_USAGE_PATH") {
            env_owned.push((
                "BPMUX_USAGE_PATH".to_string(),
                bpmux.usage_path().to_string_lossy().into_owned(),
            ));
        }
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_LOG") {
            env_owned.push((
                "BPMUX_LOG".to_string(),
                std::env::temp_dir()
                    .join("bpmux-test-diag.log")
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
        if !env_owned.iter().any(|(k, _)| k == "BPMUX_BRIEFINGS_DIR") {
            env_owned.push((
                "BPMUX_BRIEFINGS_DIR".to_string(),
                bpmux.briefings_dir_path().to_string_lossy().into_owned(),
            ));
        }
        let process = TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
            cmd.arg("bridge")
                .arg("--port")
                .arg(port.to_string())
                .arg("--name")
                .arg(&name_owned);
            if matches!(tracker_mode, TrackerMode::Fake) {
                cmd.arg("--fake-tracker");
            }
            for (k, v) in &env_owned {
                cmd.env(k, v);
            }
        }))
        .expect("spawn bridge");
        let client = NoopHandler
            .serve(process)
            .await
            .expect("MCP handshake with bridge");
        let bridge = Self { client };
        bridge.wait_until_ws_ready(name).await;
        bridge
    }

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

    pub async fn subscribe(&self, event_class: &str) -> String {
        self.call_tool(
            "subscribe",
            serde_json::json!({ "event_class": event_class }),
        )
        .await
        .expect("subscribe")["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string()
    }

    pub async fn wait_for_event(&self, subscription_id: &str, timeout: Duration) -> String {
        self.call_tool(
            "wait_for_event",
            serde_json::json!({
                "subscription_id": subscription_id,
                "timeout_ms": timeout.as_millis() as u64
            }),
        )
        .await
        .expect("wait_for_event")
        .to_string()
    }

    pub async fn call_tool(
        &self,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut params_template = rmcp::model::CallToolRequestParams::new(tool.to_string());
        params_template.arguments = args.as_object().cloned();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let mut params = rmcp::model::CallToolRequestParams::new(tool.to_string());
            params.arguments = params_template.arguments.clone();
            let result = tokio::time::timeout(
                Duration::from_secs(25),
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
            return Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)));
        }
    }

    pub async fn close(self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.client.cancel()).await;
    }
}
