#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn dev_firing_reorder_returns_structured_409_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-reorder-substrate-refusal",
            "reject if dev firing reorder_story isn't substrate-refused per workflow.toml actor_roles (#1265 closure)",
        )
        .await;

        let err = dev
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "number": target_number,
                    "position": 1,
                    "state": "ready",
                }),
            )
            .await
            .expect_err("dev should be substrate-refused at reorder axis (#1265 closure)");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"reorder\""),
            "attempted_transition names the gate; got {err}"
        );
        assert!(
            err.contains("\"pm\""),
            "allowed_roles cites the workflow.toml-configured set; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn designer_firing_reorder_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-reorder-substrate-refusal",
            "reject if designer firing reorder_story isn't substrate-refused",
        )
        .await;

        let err = designer
            .call_tool(
                "reorder_story",
                serde_json::json!({
                    "number": target_number,
                    "position": 1,
                    "state": "ready",
                }),
            )
            .await
            .expect_err("designer should be substrate-refused at reorder axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"designer\""),
            "actor_role names firing role; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn dev_firing_move_with_position_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-position-move-substrate-refusal",
            "reject if dev firing move with position isn't substrate-refused per PM-owns-queue-position canon",
        )
        .await;

        let err = dev
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": target_number,
                    "target_state": "ready",
                    "position": 1,
                    "rationale": "dev should not be able to set queue position per PM-owns-order canon",
                }),
            )
            .await
            .expect_err("dev should be substrate-refused at position-move axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"move-with-position\""),
            "attempted_transition names the gate; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}
