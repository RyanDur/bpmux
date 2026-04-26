//! Agent-side E2E tests for identity-mutation MCP tools (#476 Phase 1
//! backfill — memory archetype, identity-graph variant).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Covers: add_value / add_principle / update_principle / remove_principle.
//!
//! Per the 3-class framing — identity-mutation applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Each test uses a unique role-name to isolate the on-disk identity
//! file under `~/.bpmux/identity/<role>.json` (per #484 disk-isolation
//! gap; same workaround as commit 20 briefings tests).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_micros()
        .to_string()
}

#[test]
fn add_value_returns_identity_payload_with_new_value() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let role = "identity-add-value-test-role";
        let agent = TestBridge::connect(&bpmux, role).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let value_name = format!("TEST_VALUE_NAME_V1_{}", unique_suffix());
        let result = agent
            .call_tool(
                "add_value",
                serde_json::json!({
                    "name": value_name.clone(),
                    "description": "test value for e2e add_value coverage"
                }),
            )
            .await
            .expect("add_value should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains(&value_name),
            "identity payload includes new value name; got:\n{text}"
        );

        agent.close().await;
    });
}

#[test]
fn add_value_rejects_missing_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "identity-missing-name-role").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = agent
            .call_tool("add_value", serde_json::json!({"description": "no name"}))
            .await
            .expect_err("missing name should be rejected");

        assert!(
            err.contains("name"),
            "rejection names the missing field; got {err}"
        );

        agent.close().await;
    });
}

#[test]
fn add_value_rejects_missing_description() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "identity-missing-desc-role").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = agent
            .call_tool(
                "add_value",
                serde_json::json!({"name": "name-without-description"}),
            )
            .await
            .expect_err("missing description should be rejected");

        assert!(
            err.contains("description"),
            "rejection names the missing field; got {err}"
        );

        agent.close().await;
    });
}

#[test]
fn add_principle_after_add_value_returns_principle_in_payload() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let role = "identity-principle-roundtrip-role";
        let agent = TestBridge::connect(&bpmux, role).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let value_name = format!("ROUNDTRIP_VALUE_R1_{}", unique_suffix());
        agent
            .call_tool(
                "add_value",
                serde_json::json!({
                    "name": value_name.clone(),
                    "description": "host value for principle"
                }),
            )
            .await
            .expect("add_value should succeed");

        let principle_marker = format!("ROUNDTRIP_PRINCIPLE_MARKER_P1_{}", unique_suffix());
        let result = agent
            .call_tool(
                "add_principle",
                serde_json::json!({
                    "value_name": value_name.clone(),
                    "principle": principle_marker.clone()
                }),
            )
            .await
            .expect("add_principle should succeed");

        let text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            text.contains(&principle_marker),
            "principle preserved in identity payload via real on-disk roundtrip; got:\n{text}"
        );

        agent.close().await;
    });
}

#[test]
fn remove_principle_drops_principle_from_identity_payload() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let role = "identity-remove-principle-role";
        let agent = TestBridge::connect(&bpmux, role).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let value_name = format!("REMOVE_HOST_VALUE_{}", unique_suffix());
        agent
            .call_tool(
                "add_value",
                serde_json::json!({
                    "name": value_name.clone(),
                    "description": "host value for remove-principle test"
                }),
            )
            .await
            .expect("add_value should succeed");

        let principle_marker = format!("REMOVE_PRINCIPLE_MARKER_R1_{}", unique_suffix());
        agent
            .call_tool(
                "add_principle",
                serde_json::json!({
                    "value_name": value_name.clone(),
                    "principle": principle_marker.clone()
                }),
            )
            .await
            .expect("add_principle should succeed");

        let after_remove = agent
            .call_tool(
                "remove_principle",
                serde_json::json!({
                    "value_name": value_name.clone(),
                    "principle": principle_marker.clone()
                }),
            )
            .await
            .expect("remove_principle should succeed");

        let text = after_remove
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| after_remove.to_string());
        assert!(
            !text.contains(&principle_marker),
            "principle removed from identity payload via on-disk roundtrip; got:\n{text}"
        );

        agent.close().await;
    });
}

#[test]
fn remove_principle_rejects_missing_value_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "identity-remove-missing-vn-role").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = agent
            .call_tool("remove_principle", serde_json::json!({"principle": "any"}))
            .await
            .expect_err("missing value_name should be rejected");

        assert!(
            err.contains("value_name"),
            "rejection names the missing field; got {err}"
        );

        agent.close().await;
    });
}
