#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn pm_firing_start_story_returns_structured_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] pm-start-story-substrate-refusal",
            "reject if PM firing start_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = pm
            .call_tool(
                "start_story",
                serde_json::json!({
                    "id": target_id,
                    "pickup_paraphrase": "irrelevant — should refuse before paraphrase-check",
                    "falsifier_reframing": "irrelevant — should refuse before falsifier-check",
                }),
            )
            .await
            .expect_err("pm should be substrate-refused at start_story axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"pm\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"pickup\""),
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
fn designer_firing_start_story_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] designer-start-story-substrate-refusal",
            "reject if designer firing start_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = designer
            .call_tool(
                "start_story",
                serde_json::json!({
                    "id": target_id,
                    "pickup_paraphrase": "irrelevant",
                    "falsifier_reframing": "irrelevant",
                }),
            )
            .await
            .expect_err("designer should be substrate-refused at start_story axis");

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
