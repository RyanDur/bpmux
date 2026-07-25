#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use crate::harness::bridge::TestBridge;
use crate::harness::pty::TestBpmux;
use crate::harness::story_helpers::{create_target_story, ensure_current_iteration};

#[test]
fn move_story_via_mcp_reflects_in_get_story_immediately() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] mutation-parity-target",
            "reject if move_story success-message doesn't reflect in get_story",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "id": target_id,
                "target_state": "ready",
                "rationale": "drive #1197 regression: success-message must match canonical-store"
            }),
        )
        .await
        .expect("move_story → ready should succeed");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let story = pm
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        let workflow_state = story
            .get("workflow_state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert_eq!(
            workflow_state.to_ascii_lowercase(),
            "ready",
            "post-#1197 fix: MCP move_story success must reflect in MCP get_story \
             workflow_state immediately. Got workflow_state={workflow_state}; full story={story}"
        );

        pm.close().await;
    });
}

#[test]
fn start_story_via_mcp_reflects_in_get_story_immediately() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] start-story-parity-target",
            "reject if start_story success doesn't reflect in get_story",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({
                "id": target_id,
                "target_state": "ready",
                "rationale": "promote into ready so dev can start_story"
            }),
        )
        .await
        .expect("move_story → ready should succeed");

        tokio::time::sleep(Duration::from_millis(500)).await;

        dev.call_tool(
            "start_story",
            serde_json::json!({
                "id": target_id,
                "pickup_paraphrase": "regression test for #1197 substrate-fix",
                "falsifier_reframing": "reject if start_story success doesn't propagate"
            }),
        )
        .await
        .expect("start_story should succeed for dev");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let story = dev
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        let workflow_state = story
            .get("workflow_state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        assert!(
            matches!(
                workflow_state.to_ascii_lowercase().as_str(),
                "in progress" | "in-progress" | "in_progress"
            ),
            "post-#1197 fix: MCP start_story success must reflect in MCP get_story \
             workflow_state=in-progress (any spelling) immediately. \
             Got workflow_state={workflow_state}"
        );

        pm.close().await;
        dev.close().await;
    });
}
