//! Bridge Acceptance Test: Messages flow through the channel bridge
//!
//! Proves: human types in TUI → bridge receives via WebSocket →
//! bridge exposes message via MCP → MCP client reads it →
//! MCP client sends reply via bridge → reply appears in TUI.
//!
//! This tests the bridge as an MCP server (over stdio) that forwards
//! to/from bpmux's WebSocket endpoint. It does NOT require Claude Code
//! or an API key — it simulates what Claude Code would do by connecting
//! as an MCP client to the bridge's stdio transport.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui_testlib::ScreenState;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn read_until_contains(
    reader: &mut dyn Read,
    target: &str,
    timeout: Duration,
) -> (String, Vec<u8>) {
    let mut all_bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let start = Instant::now();

    loop {
        if start.elapsed() >= timeout {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                all_bytes.extend_from_slice(&buf[..n]);
                let mut screen = ScreenState::new(80, 24);
                screen.feed(&all_bytes);
                if screen.contents().contains(target) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut screen = ScreenState::new(80, 24);
    screen.feed(&all_bytes);
    (screen.contents(), all_bytes)
}

#[test]
fn messages_flow_through_bridge() {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");
    let port = 14119;

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
    cmd.arg("dev");
    cmd.arg("--mcp-port");
    cmd.arg(port.to_string());
    cmd.arg("--session-base");
    cmd.arg(session_base);
    cmd.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let mut writer = pair.master.take_writer().expect("take writer");

    // 2. Wait for TUI
    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("solo"), "TUI should render");

    // 3. Start the bridge as a subprocess, connect as MCP client over stdio
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use rmcp::ServiceExt;
        use rmcp::transport::child_process::{ConfigureCommandExt, TokioChildProcess};

        // Spawn bridge as child process — MCP over stdio
        let child_process =
            TokioChildProcess::new(tokio::process::Command::new(bin).configure(|cmd| {
                cmd.arg("bridge").arg("--port").arg(port.to_string());
            }))
            .expect("spawn bridge");

        // Connect as MCP client to the bridge
        let client = ().serve(child_process).await.expect("MCP handshake with bridge");

        // 4. Verify bridge exposes tools
        let tools = client.peer().list_all_tools().await.expect("list tools");
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            names.contains(&"send_message"),
            "bridge should expose send_message tool: {names:?}"
        );

        // 5. Human types a message in TUI
        writer.write_all(b"hello from human").expect("type message");
        writer.write_all(b"\r").expect("press enter");

        // Wait for message to propagate through bpmux → WebSocket → bridge
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 6. Read messages through bridge — should contain human's message
        let params = rmcp::model::CallToolRequestParams::new("read_messages");
        let result = client
            .peer()
            .call_tool(params)
            .await
            .expect("read_messages via bridge");
        let content = format!("{result:?}");
        assert!(
            content.contains("hello from human"),
            "Bridge should relay messages from bpmux. Got: {content}"
        );

        // 7. Send a reply through the bridge
        let mut params = rmcp::model::CallToolRequestParams::new("send_message");
        params.arguments = serde_json::json!({"text": "hello from bridge"})
            .as_object()
            .cloned();
        let result = client
            .peer()
            .call_tool(params)
            .await
            .expect("send_message via bridge");
        assert!(
            !result.is_error.unwrap_or(false),
            "send_message should succeed"
        );

        client.cancel().await.expect("cleanup");
    });

    // 8. Verify bridge's message appears in TUI
    let (screen, _) = read_until_contains(&mut reader, "hello from bridge", Duration::from_secs(5));
    assert!(
        screen.contains("hello from bridge"),
        "Bridge message should appear in TUI. Screen:\n{screen}"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
