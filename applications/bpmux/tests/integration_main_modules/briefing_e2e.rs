//! Agent-side E2E test for `save_briefing` + `get_briefing` MCP tool
//! pair (#476 Phase 1 backfill — memory archetype, paired write/read).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — briefing applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn save_briefing_returns_confirmation_with_role_and_bytes() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let content = "## Where I left off\n\nMid-flight on briefing E2E test.\n";
        let result = dev
            .call_tool("save_briefing", serde_json::json!({"content": content}))
            .await
            .expect("save_briefing should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("briefing saved"),
            "happy-path message confirms persistence; got {msg}"
        );
        assert!(msg.contains("dev"), "message names the role; got {msg}");
        assert!(
            msg.contains(&format!("{} bytes", content.len())),
            "message names the size for caller-visibility; got {msg}"
        );

        dev.close().await;
    });
}

#[test]
fn save_briefing_rejects_missing_content() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("save_briefing", serde_json::json!({}))
            .await
            .expect_err("missing content should be rejected");

        assert!(
            err.contains("content"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn save_briefing_get_briefing_roundtrip_preserves_content() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let role = "briefing-roundtrip-test-role";
        let dev = TestBridge::connect(&bpmux, role).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let marker = "BRIEFING_ROUNDTRIP_MARKER_B1";
        let content = format!("## Continuity\n\n{marker}\n\nMid-flight context.");
        dev.call_tool(
            "save_briefing",
            serde_json::json!({"content": content.clone()}),
        )
        .await
        .expect("save_briefing should succeed");

        let briefing = dev
            .call_tool("get_briefing", serde_json::json!({}))
            .await
            .expect("get_briefing should succeed");

        let briefing_text = briefing
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| briefing.to_string());
        assert!(
            briefing_text.contains(marker),
            "content preserved verbatim through real briefing-store; got:\n{briefing_text}"
        );
    });
}

#[test]
fn get_briefing_role_arg_overrides_default() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let pm_marker = "PM_BRIEFING_OVERRIDE_MARKER";
        pm.call_tool(
            "save_briefing",
            serde_json::json!({
                "content": format!("## PM continuity\n\n{pm_marker}")
            }),
        )
        .await
        .expect("save_briefing on pm should succeed");

        let briefing = dev
            .call_tool("get_briefing", serde_json::json!({"role": "pm"}))
            .await
            .expect("get_briefing role=pm should succeed");

        let briefing_text = briefing
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| briefing.to_string());
        assert!(
            briefing_text.contains(pm_marker),
            "role-override surfaces pm's briefing from dev's bridge; got:\n{briefing_text}"
        );

        pm.close().await;
        dev.close().await;
    });
}
