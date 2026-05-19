#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn dev_firing_close_story_returns_structured_409_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-close-substrate-refusal",
            "reject if dev firing close_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "superseded by some-coherent-substantive-cite",
                }),
            )
            .await
            .expect_err("dev should be substrate-refused at close axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"close\""),
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
fn designer_firing_close_story_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-close-substrate-refusal",
            "reject if designer firing close_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = designer
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "superseded by some-coherent-substantive-cite",
                }),
            )
            .await
            .expect_err("designer should be substrate-refused at close axis");

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
fn pm_firing_close_story_with_short_reason_returns_structured_409_reason_cite_missing() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] short-reason-substrate-refusal",
            "reject if short close_story reason isn't substrate-refused",
        )
        .await;

        let err = pm
            .call_tool(
                "close_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "x",
                }),
            )
            .await
            .expect_err("short reason should be substrate-refused");

        assert!(
            err.contains("\"missing_evidence\":[\"reason-cite\"]"),
            "missing_evidence cites reason-cite; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"close\""),
            "attempted_transition names the gate; got {err}"
        );

        pm.close().await;
    });
}
