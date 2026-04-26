//! Agent-side E2E test for `label_story` MCP tool (#476 Phase 1
//! backfill — mutation archetype, add/remove variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — label_story applicability =
//! {agent: yes, human: yes (badge-toggle Electron surface),
//! external: yes (gh issue label)}. This test covers ONLY agent-side.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn label_story_add_returns_message_naming_label() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] label-add-target",
            "reject if label_story add doesn't fire on the target",
        )
        .await;

        let result = pm
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": target_number,
                    "label": "blocked",
                    "operation": "add"
                }),
            )
            .await
            .expect("label_story add should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("labeled") && msg.contains("blocked"),
            "happy-path message names label affordance + label-name; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn label_story_remove_returns_message_naming_label() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] label-remove-target",
            "reject if label_story remove doesn't fire",
        )
        .await;

        let result = pm
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": target_number,
                    "label": "blocked",
                    "operation": "remove"
                }),
            )
            .await
            .expect("label_story remove should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("unlabeled") && msg.contains("blocked"),
            "happy-path message names remove affordance + label-name; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn label_story_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-cannot-label-target",
            "reject if dev can label a story bypassing pm-lane gate",
        )
        .await;

        let err = dev
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": target_number,
                    "label": "blocked",
                    "operation": "add"
                }),
            )
            .await
            .expect_err("dev should be rejected by label pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn label_story_rejects_invalid_operation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] invalid-operation-target",
            "reject if invalid operation is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "label_story",
                serde_json::json!({
                    "number": target_number,
                    "label": "blocked",
                    "operation": "toggle"
                }),
            )
            .await
            .expect_err("unknown operation should be rejected");

        assert!(
            err.contains("operation") && (err.contains("add") || err.contains("remove")),
            "rejection names the allowed operations; got {err}"
        );

        pm.close().await;
    });
}
