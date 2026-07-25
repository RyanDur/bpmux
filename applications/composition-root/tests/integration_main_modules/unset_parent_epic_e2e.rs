#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn unset_parent_epic_returns_message_naming_target() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] unset-parent-target",
            "reject if unset_parent_epic doesn't fire ParentEpicUnset event",
        )
        .await;

        let result = pm
            .call_tool("unset_parent_epic", serde_json::json!({"id": target_id}))
            .await
            .expect("unset_parent_epic should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_id}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("ungrouped"),
            "happy-path message names the unset affordance; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn unset_parent_epic_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool("unset_parent_epic", serde_json::json!({"id": 100}))
            .await
            .expect_err("dev should be rejected by unset_parent_epic pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn unset_parent_epic_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("unset_parent_epic", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("'id'"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
