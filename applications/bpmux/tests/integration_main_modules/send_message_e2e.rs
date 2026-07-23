#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn send_message_rejects_missing_text() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("send_message", serde_json::json!({}))
            .await
            .expect_err("missing text should be rejected");

        assert!(
            err.contains("text"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
