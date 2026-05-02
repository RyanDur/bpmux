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

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

const WAIT_PRESENCE_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn messages_flow_through_bridge() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let bridge = TestBridge::connect(&bpmux, "bridge").await;

        bpmux.type_line("hello from human");
        bridge
            .wait_for_message("hello from human", WAIT_PRESENCE_TIMEOUT)
            .await;

        bridge.send("hello from bridge").await;

        bridge.close().await;
    });

    assert!(
        bpmux.wait_for_tui("hello from bridge", WAIT_PRESENCE_TIMEOUT),
        "Bridge message should appear in TUI. Screen:\n{}",
        bpmux.tui()
    );
}
