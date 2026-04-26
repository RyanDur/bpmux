//! Agent-side E2E test for `read_messages` MCP tool (#476 Phase 1
//! backfill — read archetype, message-log variant). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — read_messages applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn read_messages_returns_recent_message_after_send() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "READ_MESSAGES_MARKER_R1";
        dev.send(marker).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let result = dev
            .call_tool("read_messages", serde_json::json!({}))
            .await
            .expect("read_messages should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains(marker),
            "read_messages reflects recently-sent message via real bpmux-server roundtrip; got:\n{text}"
        );

        dev.close().await;
    });
}
