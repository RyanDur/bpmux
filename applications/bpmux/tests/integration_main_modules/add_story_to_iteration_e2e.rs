#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn add_story_to_iteration_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "add_story_to_iteration",
                serde_json::json!({
                    "iteration_id": 1397,
                    "story_id": 1500,
                    "position": 100
                }),
            )
            .await
            .expect_err("dev should be rejected by add-to-iteration pm-lane gate");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"]
                == "workflow-role-not-allowed",
            "structured kind cites workflow-role-not-allowed; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["attempted_transition"]
                == "add-to-iteration",
            "rejection cites attempted_transition; got {err}"
        );

        dev.close().await;
    });
}

#[test]
fn add_story_to_iteration_rejects_missing_iteration_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "add_story_to_iteration",
                serde_json::json!({"story_id": 1500, "position": 100}),
            )
            .await
            .expect_err("missing iteration_id should be rejected");

        assert!(
            err.contains("iteration_id"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn add_story_to_iteration_rejects_missing_story_id() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "add_story_to_iteration",
                serde_json::json!({"iteration_id": 1397, "position": 100}),
            )
            .await
            .expect_err("missing story_id should be rejected");

        assert!(
            err.contains("story_id"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn add_story_to_iteration_rejects_missing_position() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "add_story_to_iteration",
                serde_json::json!({"iteration_id": 1397, "story_id": 1500}),
            )
            .await
            .expect_err("missing position should be rejected");

        assert!(
            err.contains("position"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
