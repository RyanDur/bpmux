//! Peer Count Test: TUI status bar shows connected peer count
//!
//! Proves: when a WebSocket client connects and identifies,
//! the TUI status bar changes from "solo" to "1 connected".

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::read_until_contains;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::time::Duration;

#[test]
fn status_bar_shows_peer_count() {
    let bin = env!("CARGO_BIN_EXE_bpmux");
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let session_base = temp_dir.path().to_str().expect("temp path");
    let port = 14122;

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
    let _writer = pair.master.take_writer().expect("take writer");

    // 2. Wait for TUI — should show "solo"
    let (screen, _) = read_until_contains(&mut reader, "solo", Duration::from_secs(5));
    assert!(screen.contains("solo"), "TUI should show solo initially");

    // 3. Connect a WebSocket client and identify
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;

        let ws_url = format!("ws://127.0.0.1:{port}/ws");

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
                serde_json::json!({"type": "identify", "name": "test-peer"})
                    .to_string()
                    .into(),
            ))
            .await
            .expect("send identify");

        // Read welcome
        ws_rx.next().await.expect("receive").expect("welcome msg");

        // 4. Wait for status bar to update — should show "1 connected"
        let (screen, _) = tokio::task::spawn_blocking(move || {
            read_until_contains(&mut reader, "1 connected", Duration::from_secs(5))
        })
        .await
        .expect("spawn_blocking");

        assert!(
            screen.contains("1 connected"),
            "Status bar should show '1 connected'. Screen:\n{screen}"
        );
    });

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
