//! Story 2 Acceptance Test: Human and AI share a conversation
//!
//! End-to-end test using portable-pty for TUI interaction and rmcp for MCP client.
//! Proves: human types in TUI → message appears, AI sends via MCP → message appears
//! in TUI, AI reads via MCP → gets both messages.

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
fn human_and_ai_share_a_conversation() {
    let bin = env!("CARGO_BIN_EXE_bpmux");

    // Isolated temp directory for this test's session
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");

    // Use a non-default port to avoid conflicts
    let mcp_port = 14117;

    // Spawn bpmux in a PTY
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
    cmd.arg("ryan");
    cmd.arg("--mcp-port");
    cmd.arg(mcp_port.to_string());
    cmd.arg("--session-base");
    cmd.arg(session_base);
    cmd.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn bpmux");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let mut writer = pair.master.take_writer().expect("take writer");

    // 1. Verify TUI renders with empty chat
    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("bpmux"), "TUI should show bpmux title");
    assert!(screen.contains("solo"), "TUI should show solo status");
    assert!(screen.contains(">"), "TUI should show input prompt");

    // 2. Human types a message
    writer.write_all(b"hello from human").expect("type message");
    writer.write_all(b"\r").expect("press enter");

    // 3. Verify human message appears in TUI
    let (screen, _) = read_until_contains(&mut reader, "hello from human", Duration::from_secs(3));
    assert!(
        screen.contains("hello from human"),
        "Human message should appear in TUI. Screen:\n{screen}"
    );
    assert!(
        screen.contains("ryan:human"),
        "Sender identity should show. Screen:\n{screen}"
    );

    eprintln!("Steps 1-3 passed. Connecting MCP client...");

    // 4. Connect MCP client over HTTP
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
        use std::time::Duration;

        let mcp_url = format!("http://127.0.0.1:{mcp_port}/mcp");

        // Retry connection — HTTP server may need a moment
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match reqwest::get(&mcp_url).await {
                    Ok(_) => return,
                    Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                }
            }
        })
        .await
        .expect("MCP HTTP server should be reachable");

        // 5. MCP handshake over HTTP
        use rmcp::ServiceExt;
        use rmcp::transport::StreamableHttpClientTransport;

        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(mcp_url),
        );
        let client = ().serve(transport).await.expect("MCP handshake");

        // 6. Verify tools exist
        let tools = client.peer().list_all_tools().await.expect("list tools");
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            names.contains(&"send_message"),
            "send_message tool missing: {names:?}"
        );
        assert!(
            names.contains(&"read_messages"),
            "read_messages tool missing: {names:?}"
        );

        // 7. AI sends a message
        let mut params = rmcp::model::CallToolRequestParams::new("send_message");
        params.arguments = serde_json::json!({"text": "hello from ai"})
            .as_object()
            .cloned();
        let result = client.peer().call_tool(params).await.expect("send_message");
        assert!(
            !result.is_error.unwrap_or(false),
            "send_message should succeed"
        );

        // 8. AI reads messages — should see both
        let params = rmcp::model::CallToolRequestParams::new("read_messages");
        let result = client
            .peer()
            .call_tool(params)
            .await
            .expect("read_messages");
        let content = format!("{result:?}");
        assert!(
            content.contains("hello from human"),
            "read_messages should contain human message. Got: {content}"
        );
        assert!(
            content.contains("hello from ai"),
            "read_messages should contain AI message. Got: {content}"
        );

        client.cancel().await.expect("cleanup");
    });

    // 9. Verify AI message appears in TUI
    let (screen, _) = read_until_contains(&mut reader, "hello from ai", Duration::from_secs(3));
    assert!(
        screen.contains("hello from ai"),
        "AI message should appear in TUI. Screen:\n{screen}"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
