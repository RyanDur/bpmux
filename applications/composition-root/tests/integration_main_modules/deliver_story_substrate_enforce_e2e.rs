#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn pm_firing_deliver_returns_structured_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] pm-deliver-substrate-refusal",
            "reject if PM firing deliver_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = pm
            .call_tool(
                "deliver_story",
                serde_json::json!({
                    "id": target_id,
                    "verified_observation": "irrelevant — should refuse before observation-check",
                }),
            )
            .await
            .expect_err("pm should be substrate-refused at deliver-axis");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "workflow-role-not-allowed",
            "structured kind discriminator present; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["actor_role"] == "pm",
            "actor_role names firing role; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["attempted_transition"] == "deliver",
            "attempted_transition names the gate; got {err}"
        );
        assert!(
            err.contains("\"dev\""),
            "allowed_roles cites the workflow.toml-configured set; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn designer_firing_deliver_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-deliver-substrate-refusal",
            "reject if designer firing deliver_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = designer
            .call_tool(
                "deliver_story",
                serde_json::json!({
                    "id": target_id,
                    "verified_observation": "irrelevant — should refuse before observation-check",
                }),
            )
            .await
            .expect_err("designer should be substrate-refused at deliver-axis");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "workflow-role-not-allowed",
            "structured kind discriminator present; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["actor_role"] == "designer",
            "actor_role names firing role; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["attempted_transition"] == "deliver",
            "attempted_transition names the gate; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}
