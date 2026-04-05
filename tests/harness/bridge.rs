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

impl TestBridge {
    /// Spawn a bridge subprocess with `--name` and connect an MCP client.
    pub async fn connect(bpmux: &TestBpmux, name: &str) -> Self {
        let bin = env!("CARGO_BIN_EXE_bpmux");
        let port = bpmux.port();
        let name_owned = name.to_string();
        let process = TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
            cmd.arg("bridge")
                .arg("--port")
                .arg(port.to_string())
                .arg("--name")
                .arg(&name_owned);
        }))
        .expect("spawn bridge");
        let capture = NotificationCapture::default();
        let client = capture
            .clone()
            .serve(process)
            .await
            .expect("MCP handshake with bridge");
        Self { client, capture }
    }

    /// Send a message through `send_message`, retrying until the bridge's
    /// WebSocket to bpmux is connected.
    pub async fn send(&self, text: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
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
