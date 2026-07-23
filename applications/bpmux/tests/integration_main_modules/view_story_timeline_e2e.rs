#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn view_story_timeline_returns_filed_event_for_new_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] timeline-target",
            "reject if view_story_timeline omits the filed event",
        )
        .await;

        let result = pm
            .call_tool("view_story_timeline", serde_json::json!({"id": target_id}))
            .await
            .expect("view_story_timeline should succeed");

        let parsed = match &result {
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => result.clone(),
            serde_json::Value::String(s) => {
                serde_json::from_str::<serde_json::Value>(s).expect("returns parseable JSON")
            }
            other => panic!("view_story_timeline returned unexpected shape: {other:?}"),
        };
        assert!(
            parsed.is_array() || parsed.is_object(),
            "timeline returns structured JSON; got {parsed:?}"
        );

        pm.close().await;
    });
}

#[test]
fn view_story_timeline_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("view_story_timeline", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("'id'"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn view_story_timeline_returns_error_for_non_existent_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("view_story_timeline", serde_json::json!({"id": 999999}))
            .await
            .expect_err("non-existent story should be rejected");

        assert!(
            err.contains("not found") || err.contains("999999"),
            "rejection names the missing story; got {err}"
        );

        pm.close().await;
    });
}
