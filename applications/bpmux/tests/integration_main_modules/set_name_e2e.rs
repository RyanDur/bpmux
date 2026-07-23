#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn set_name_returns_confirmation_with_new_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("set_name", serde_json::json!({"name": "RENAME_MARKER_N1"}))
            .await
            .expect("set_name should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("name set to"),
            "happy-path message confirms rename; got {msg}"
        );
        assert!(
            msg.contains("RENAME_MARKER_N1"),
            "message echoes the new name verbatim; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn set_name_rejects_missing_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("set_name", serde_json::json!({}))
            .await
            .expect_err("missing name should be rejected");

        assert!(
            err.contains("name"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
