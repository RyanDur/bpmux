//! Story 3 Acceptance Test: WebSocket client participates in conversation
//!
//! Proves: WebSocket client connects → receives human's message →
//! sends a response → response appears in TUI. Uses real collaborators,
//! no mocks, no external APIs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::read_until_contains;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::Write;
use std::time::Duration;

#[test]
fn ws_client_participates_in_conversation() {
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

    // 3. Connect WebSocket client, then have human type, then receive via WS.
    //    All WS operations happen inside one block_on so the runtime stays active
    //    and can buffer incoming frames while we do PTY I/O on a spawned thread.
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;

        let ws_url = format!("ws://127.0.0.1:{port}/ws");

        // Connect (retry until server is ready)
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
                serde_json::json!({"type": "identify", "name": "bot"})
                    .to_string()
                    .into(),
            ))
            .await
            .expect("send identify");

        // Receive welcome
        let welcome = ws_rx.next().await.expect("receive").expect("welcome msg");
        let welcome: serde_json::Value =
            serde_json::from_str(welcome.to_text().expect("text")).expect("parse welcome");
        assert_eq!(
            welcome["type"], "welcome",
            "First response should be welcome"
        );

        // Human types a message on a blocking thread (PTY I/O is synchronous)
        let pty_done = tokio::task::spawn_blocking(move || {
            writer.write_all(b"hello from human").expect("type message");
            writer.write_all(b"\r").expect("press enter");
            writer
        });

        // Receive the human's message via WS push (skip system messages like join notifications)
        let msg = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let frame = ws_rx
                    .next()
                    .await
                    .expect("stream not ended")
                    .expect("message ok");
                let parsed: serde_json::Value =
                    serde_json::from_str(frame.to_text().expect("text")).expect("parse message");
                if parsed["type"] == "message" && parsed["text"] != "bot:ws joined" {
                    return parsed;
                }
            }
        })
        .await
        .expect("should receive human message within timeout");
        let msg = msg;
        assert_eq!(msg["type"], "message");
        assert_eq!(msg["text"], "hello from human");

        // Send a response
        ws_tx
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::json!({"type": "send", "text": "hello from bot"})
                    .to_string()
                    .into(),
            ))
            .await
            .expect("send reply");

        // Wait for PTY write to finish, get writer back for cleanup
        let _writer = pty_done.await.expect("pty write thread");
    });

    // 4. Verify bot response appears in TUI
    let (screen, _) = read_until_contains(&mut reader, "hello from bot", Duration::from_secs(5));
    assert!(
        screen.contains("hello from bot"),
        "Bot response should appear in TUI. Screen:\n{screen}"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
