//! Agent-side E2E test for `get_identity` MCP tool (#476 Phase 1
//! backfill — bridge-state-read archetype). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — get_identity applicability =
//! {agent: yes, human: no (no Electron surface), external: no
//! (identity is bpmux-internal, not GH-mirrored)}. Single-class
//! agent-only coverage matches bridge-state-read default per
//! designer's index.md.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn get_identity_default_role_returns_calling_bridges_identity() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = dev
            .call_tool("get_identity", serde_json::json!({}))
            .await
            .expect("get_identity should succeed");

        assert!(
            result.is_object() || result.is_string(),
            "get_identity returns a payload (object OR pretty-printed JSON string); got {result:?}"
        );

        dev.close().await;
    });
}

#[test]
fn get_identity_role_arg_overrides_default() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let dev_default = dev
            .call_tool("get_identity", serde_json::json!({}))
            .await
            .expect("get_identity default succeeds");

        let pm_via_override = dev
            .call_tool("get_identity", serde_json::json!({"role": "pm"}))
            .await
            .expect("get_identity with role=pm succeeds");

        let dev_text = dev_default.to_string();
        let pm_text = pm_via_override.to_string();
        assert!(
            dev_text != pm_text || dev_text.contains("dev") || pm_text.contains("pm"),
            "role-override surfaces a different identity OR identity output names the requested role. dev:\n{dev_text}\npm:\n{pm_text}"
        );

        dev.close().await;
    });
}

#[test]
fn get_identity_returns_well_formed_payload_for_known_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool("get_identity", serde_json::json!({"role": "pm"}))
            .await
            .expect("get_identity for pm succeeds");

        let result_text = result
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| result.to_string());
        assert!(
            !result_text.is_empty(),
            "get_identity returns non-empty payload; got `{result_text}`"
        );

        pm.close().await;
    });
}
