#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn share_message_form_2_fresh_text_returns_confirmation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "share_message",
                serde_json::json!({
                    "scope": "all",
                    "text": "fresh broadcast via share_message Form 2"
                }),
            )
            .await
            .expect("share_message Form 2 should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains("shared fresh message to all"),
            "Form 2 returns the fresh-share confirmation; got `{text}`"
        );

        pm.close().await;
    });
}

#[test]
fn share_message_rejects_missing_scope() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("share_message", serde_json::json!({"text": "any"}))
            .await
            .expect_err("missing scope should be rejected");

        assert!(
            err.contains("scope"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
