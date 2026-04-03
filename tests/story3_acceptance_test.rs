//! Story 3 Acceptance Test: AI responds to human in conversation
//!
//! End-to-end test proving: human types in TUI → bot receives via WebSocket →
//! bot calls Claude API → response appears in TUI.

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
#[ignore] // Requires ANTHROPIC_API_KEY and a running bpmux — run with: cargo test --test story3_acceptance_test -- --ignored
fn ai_responds_to_human() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping story3: ANTHROPIC_API_KEY not set");
        return;
    }

    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");
    let port = 14118;

    // 1. Spawn bpmux in a PTY
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

    // 2. Wait for TUI to render
    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("solo"), "TUI should show solo status");

    // 3. Connect bot via WebSocket
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let bot_handle = rt.spawn(async move {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;

        let ws_url = format!("ws://127.0.0.1:{port}/ws");

        // Retry connection — server may need a moment
        let ws_stream = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match connect_async(&ws_url).await {
                    Ok((stream, _)) => return stream,
                    Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                }
            }
        })
        .await
        .expect("WebSocket should connect");

        let (mut ws_tx, mut ws_rx) = ws_stream.split();

        // Identify
        ws_tx
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({"type": "identify", "name": "claude"}).to_string().into(),
            ))
            .await
            .expect("send identify");

        // Wait for welcome
        let welcome = ws_rx.next().await.expect("receive").expect("welcome msg");
        let welcome: serde_json::Value =
            serde_json::from_str(welcome.to_text().expect("text")).expect("parse welcome");
        assert_eq!(welcome["type"], "welcome", "First response should be welcome");

        // Listen for messages and respond to @claude mentions
        while let Some(Ok(frame)) = ws_rx.next().await {
            if let Ok(text) = frame.to_text() {
                let msg: serde_json::Value = match serde_json::from_str(text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if msg["type"] != "message" {
                    continue;
                }

                let from = msg["from"].as_str().unwrap_or("");
                let body = msg["text"].as_str().unwrap_or("");

                // Loop prevention
                if from.contains("claude") {
                    continue;
                }

                // Trigger: respond when mentioned
                if !body.contains("@claude") {
                    continue;
                }

                // Call Claude API
                let api_key =
                    std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
                let client = reqwest::Client::new();
                let response = client
                    .post("https://api.anthropic.com/v1/messages")
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&serde_json::json!({
                        "model": "claude-sonnet-4-20250514",
                        "max_tokens": 256,
                        "system": "You are a coding peer in a pair programming session. Be concise. One short sentence.",
                        "messages": [{"role": "user", "content": body}]
                    }))
                    .send()
                    .await
                    .expect("Claude API call");

                let result: serde_json::Value =
                    response.json().await.expect("parse Claude response");
                let reply = result["content"][0]["text"]
                    .as_str()
                    .unwrap_or("(no response)");

                // Send response back to bpmux
                ws_tx
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::json!({"type": "send", "text": reply}).to_string().into(),
                    ))
                    .await
                    .expect("send reply");

                // One response is enough for the test
                break;
            }
        }
    });

    // 4. Human types a message mentioning @claude
    // Give the bot a moment to connect
    std::thread::sleep(Duration::from_secs(2));
    writer
        .write_all(b"@claude what is 2+2?")
        .expect("type message");
    writer.write_all(b"\r").expect("press enter");

    // 5. Verify human message appears
    let (screen, _) =
        read_until_contains(&mut reader, "@claude what is 2+2?", Duration::from_secs(3));
    assert!(
        screen.contains("@claude what is 2+2?"),
        "Human message should appear. Screen:\n{screen}"
    );

    // 6. Wait for bot response to appear in TUI
    // The bot calls the real Claude API, so give it time
    let (screen, _) = read_until_contains(&mut reader, "claude:ws", Duration::from_secs(30));
    assert!(
        screen.contains("claude:ws"),
        "Bot response should appear in TUI with claude:ws sender. Screen:\n{screen}"
    );

    // Cleanup
    rt.block_on(async {
        bot_handle.abort();
    });
    let _ = child.kill();
    let _ = child.wait();
}
