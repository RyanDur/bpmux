#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::mock_gh::{issue_json, mock_story_mutations};
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use httpmock::{Method, MockServer};
use serde_json::Value;
use std::time::Duration;

#[test]
fn set_parent_epic_rejects_non_epic_parent() {
    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let parent = issue_json(10, "[Feature] not-an-epic-parent", &[], None);
    let child = issue_json(20, "[Feature] would-be-child", &[], None);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![parent.clone(), child.clone()]).to_string());
    });
    mock_story_mutations(&server, 10, &parent);
    mock_story_mutations(&server, 20, &child);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let err = pm
            .call_tool(
                "set_parent_epic",
                serde_json::json!({"id": 20, "epic_id": 10}),
            )
            .await
            .expect_err("non-Epic parent should be rejected");

        assert!(
            err.contains("not an Epic") || err.contains("Epic"),
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

        let target_id = create_target_story(
            &pm,
            "[Feature] self-parent-target",
            "reject if a story can be its own parent",
        )
        .await;

        let err = pm
            .call_tool(
                "set_parent_epic",
                serde_json::json!({
                    "id": target_id,
                    "epic_id": target_id
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
                    "id": 100,
                    "epic_id": 200
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
            .call_tool("set_parent_epic", serde_json::json!({"id": 100}))
            .await
            .expect_err("missing epic_number should be rejected");

        assert!(
            err.contains("epic_id"),
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
            .call_tool("set_parent_epic", serde_json::json!({"epic_id": 200}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("'id'"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
