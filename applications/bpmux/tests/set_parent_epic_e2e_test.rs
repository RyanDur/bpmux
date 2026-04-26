//! Agent-side E2E test for `set_parent_epic` MCP tool (#476 Phase 1
//! backfill — mutation archetype, parent-epic-relationship variant).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — set_parent_epic applicability =
//! {agent: yes, human: yes (Electron Epic-grouping surface),
//! external: yes (gh issue label parent-epic-#N)}. This test covers
//! ONLY agent-side; human-side + external follow.
//!
//! **Happy-path falsifier deferred**: per #457 walk-time F1 cascade,
//! `story_type_from_labels_or_title` doesn't detect Epic from
//! `[Epic]` title or `epic` label. No standard agent-flow produces a
//! StoryType::Epic story today, so the positive-case "set parent IS
//! Epic, accept" is structurally untestable until F1 resolves.
//! Negative-case "parent is NOT Epic, reject" tested here.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn set_parent_epic_rejects_non_epic_parent() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let parent_number = create_target_story(
            &pm,
            "[Feature] not-an-epic-parent",
            "reject if non-Epic story is silently accepted as parent",
        )
        .await;
        let child_number = create_target_story(
            &pm,
            "[Feature] would-be-child",
            "reject if child references non-Epic parent",
        )
        .await;

        let err = pm
            .call_tool(
                "set_parent_epic",
                serde_json::json!({
                    "number": child_number,
                    "epic_number": parent_number
                }),
            )
            .await
            .expect_err("non-Epic parent should be rejected");

        assert!(
            err.contains("not an Epic"),
            "rejection cites Epic-type-mismatch; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn set_parent_epic_rejects_self_parent() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] self-parent-target",
            "reject if a story can be its own parent",
        )
        .await;

        let err = pm
            .call_tool(
                "set_parent_epic",
                serde_json::json!({
                    "number": target_number,
                    "epic_number": target_number
                }),
            )
            .await
            .expect_err("self-parent should be rejected");

        assert!(
            err.contains("own parent"),
            "rejection cites self-parent disallowed; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn set_parent_epic_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "set_parent_epic",
                serde_json::json!({
                    "number": 100,
                    "epic_number": 200
                }),
            )
            .await
            .expect_err("dev should be rejected by set_parent_epic pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn set_parent_epic_rejects_missing_epic_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("set_parent_epic", serde_json::json!({"number": 100}))
            .await
            .expect_err("missing epic_number should be rejected");

        assert!(
            err.contains("epic_number"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn set_parent_epic_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("set_parent_epic", serde_json::json!({"epic_number": 200}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
