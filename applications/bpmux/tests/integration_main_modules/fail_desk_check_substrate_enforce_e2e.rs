#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn dev_firing_fail_desk_check_returns_structured_409_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-fail-desk-check-substrate-refusal",
            "reject if dev firing fail_desk_check isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "id": target_id,
                    "reason": "observed: ATP file foo.rs has zero assertions, refutes clause (b)",
                }),
            )
            .await
            .expect_err("dev should be substrate-refused at fail-desk-check axis");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"fail-desk-check\""),
            "attempted_transition names the gate; got {err}"
        );
        assert!(
            err.contains("\"designer\""),
            "allowed_roles cites the workflow.toml-configured set; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn designer_firing_fail_desk_check_with_empty_reason_returns_structured_409_falsifier_cite_missing()
{
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] empty-reason-substrate-refusal",
            "reject if empty fail_desk_check reason isn't substrate-refused",
        )
        .await;

        let err = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "id": target_id,
                    "reason": "",
                }),
            )
            .await
            .expect_err("empty reason should be substrate-refused");

        assert!(
            err.contains("\"kind\":\"workflow-gate-mismatch\""),
            "structured kind cites workflow-gate-mismatch; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"falsifier-cite\"]"),
            "missing_evidence cites falsifier-cite; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"fail-desk-check\""),
            "attempted_transition names the gate; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn designer_firing_fail_desk_check_with_generic_reason_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] generic-reason-substrate-refusal",
            "reject if short generic fail_desk_check reason isn't substrate-refused",
        )
        .await;

        let err = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "id": target_id,
                    "reason": "failed",
                }),
            )
            .await
            .expect_err("generic short reason should be substrate-refused");

        assert!(
            err.contains("\"missing_evidence\":[\"falsifier-cite\"]"),
            "missing_evidence cites falsifier-cite; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}
