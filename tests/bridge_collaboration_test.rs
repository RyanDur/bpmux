//! Agent-to-agent collaboration test: unmentioned messages still notify.
//!
//! Proves: when agent A sends a message that does not @mention agent B,
//! agent B still receives a channel notification for it. This removes the
//! parallel-monologue failure mode where agents would reply to a human
//! simultaneously and never see each other's replies.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::read_until_contains;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rmcp::model::CustomNotification;
use rmcp::service::NotificationContext;
use rmcp::{ClientHandler, RoleClient};

#[derive(Clone, Default)]
struct NotificationCapture {
    received: Arc<Mutex<Vec<String>>>,
}

impl ClientHandler for NotificationCapture {
    fn on_custom_notification(
        &self,
        notification: CustomNotification,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let received = self.received.clone();
        async move {
            if notification.method == "notifications/claude/channel"
                && let Some(params) = notification.params
                && let Some(content) = params.get("content").and_then(|v| v.as_str())
            {
                received.lock().unwrap().push(content.to_string());
            }
        }
    }
}

#[test]
fn unmentioned_agent_messages_notify_other_agents() {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");
    let port = common::free_port();

    // Start bpmux in a PTY.
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
    cmd.arg("human");
    cmd.arg("--mcp-port");
    cmd.arg(port.to_string());
    cmd.arg("--session-base");
    cmd.arg(session_base);
    cmd.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let _writer = pair.master.take_writer().expect("take writer");

    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("solo"), "TUI should render");

    // Drain PTY output in the background so TUI can keep writing.
    let mut drain_reader = pair.master.try_clone_reader().expect("clone reader 2");
    let drainer = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n) = std::io::Read::read(&mut drain_reader, &mut buf) {
            if n == 0 {
                break;
            }
        }
    });

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use rmcp::ServiceExt;
        use rmcp::transport::child_process::{ConfigureCommandExt, TokioChildProcess};

        // PM bridge with a notification-capturing client.
        let pm_capture = NotificationCapture::default();
        let pm_process =
            TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
                cmd.arg("bridge")
                    .arg("--port")
                    .arg(port.to_string())
                    .arg("--name")
                    .arg("pm");
            }))
            .expect("spawn pm bridge");
        let pm_client = pm_capture
            .clone()
            .serve(pm_process)
            .await
            .expect("MCP handshake with pm bridge");

        // Dev bridge that sends a message without @mentioning pm.
        let dev_process =
            TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
                cmd.arg("bridge")
                    .arg("--port")
                    .arg(port.to_string())
                    .arg("--name")
                    .arg("dev");
            }))
            .expect("spawn dev bridge");
        let dev_client = ().serve(dev_process).await.expect("MCP handshake with dev bridge");

        // Retry send until dev bridge's WebSocket is connected.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let mut params = rmcp::model::CallToolRequestParams::new("send_message");
                params.arguments = serde_json::json!({"text": "starting the implementation"})
                    .as_object()
                    .cloned();
                let result = dev_client
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
        .expect("dev bridge should connect");

        // Wait for pm to receive the notification from dev.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let seen = pm_capture.received.lock().unwrap().clone();
                if seen
                    .iter()
                    .any(|s| s.contains("starting the implementation"))
                {
                    return seen;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("pm should receive a notification for dev's unmentioned message");

        let _ = tokio::time::timeout(Duration::from_secs(2), dev_client.cancel()).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), pm_client.cancel()).await;
    });
    let _ = child.kill();
    let _ = child.wait();
    let _ = drainer.join();
}
