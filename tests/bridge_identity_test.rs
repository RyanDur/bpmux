//! Bridge Identity Test: Two bridges with different names show distinct identities
//!
//! Proves: when two bridge clients connect with --name dev and --name pm,
//! messages from each appear with their respective identity in the TUI.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use harness::pty::read_until_contains;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::time::Duration;

/// Retry send_message until the bridge's WebSocket connection is ready.
async fn send_when_ready(client: &rmcp::service::RunningService<rmcp::RoleClient, ()>, text: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut params = rmcp::model::CallToolRequestParams::new("send_message");
            params.arguments = serde_json::json!({"text": text}).as_object().cloned();
            let result = client.peer().call_tool(params).await.expect("tool call");
            if !result.is_error.unwrap_or(false) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("bridge should connect within timeout");
}

#[test]
fn two_bridges_show_distinct_identities() {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");
    let port = harness::port::free_port();

    // 1. Start bpmux in a PTY
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

    // 2. Wait for TUI
    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("solo"), "TUI should render");

    // 3. Start two bridges with different names and send messages
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use rmcp::ServiceExt;
        use rmcp::transport::child_process::{ConfigureCommandExt, TokioChildProcess};

        // Bridge 1: --name dev
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

        // Bridge 2: --name pm
        let pm_process =
            TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
                cmd.arg("bridge")
                    .arg("--port")
                    .arg(port.to_string())
                    .arg("--name")
                    .arg("pm");
            }))
            .expect("spawn pm bridge");

        let pm_client = ().serve(pm_process).await.expect("MCP handshake with pm bridge");

        // 4. Send from dev — retries until WebSocket is connected
        send_when_ready(&dev_client, "hello from dev").await;

        // 5. Send from pm — retries until WebSocket is connected
        send_when_ready(&pm_client, "hello from pm").await;

        dev_client.cancel().await.expect("cleanup dev");
        pm_client.cancel().await.expect("cleanup pm");
    });

    // 6. Verify both identities appear distinctly in the TUI
    let (screen, _) = read_until_contains(&mut reader, "hello from pm", Duration::from_secs(5));

    assert!(
        screen.contains("dev:") && screen.contains("hello from dev"),
        "Dev bridge should show as 'dev' in TUI. Screen:\n{screen}"
    );
    assert!(
        screen.contains("pm:") && screen.contains("hello from pm"),
        "PM bridge should show as 'pm' in TUI. Screen:\n{screen}"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
