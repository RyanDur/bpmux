#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, drive_to_delivered};
use std::time::Duration;

#[test]
fn pm_firing_reject_without_reason_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] reject-missing-reason",
            "reject if reject_story without reason-cite isn't substrate-refused",
        )
        .await;

        let err = pm
            .call_tool("reject_story", serde_json::json!({"id": target_id}))
            .await
            .expect_err("reject without reason should be refused");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"]
                == "workflow-gate-mismatch",
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"reason-cite\"]"),
            "rejection cites missing reason-cite evidence; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn pm_firing_reject_with_empty_reason_returns_structured() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] reject-empty-reason",
            "reject if empty-string reason silently passes substrate-gate",
        )
        .await;

        let err = pm
            .call_tool(
                "reject_story",
                serde_json::json!({"id": target_id, "reason": "   "}),
            )
            .await
            .expect_err("reject with whitespace-only reason should be refused");

        assert!(
            err.contains("\"missing_evidence\":[\"reason-cite\"]"),
            "trimmed-empty reason treated as missing; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn dev_firing_reject_returns_workflow_role_not_allowed() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-reject-substrate-refusal",
            "reject if dev firing reject_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "id": target_id,
                    "reason": "dev attempting unauthorized reject"
                }),
            )
            .await
            .expect_err("dev should be substrate-refused");

        assert!(
            crate::harness::story_helpers::structured_error(&err)["kind"] == "workflow-role-not-allowed",
            "rejection cites structured kind; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["actor_role"] == "dev",
            "rejection cites firing role; got {err}"
        );
        assert!(
            crate::harness::story_helpers::structured_error(&err)["attempted_transition"] == "reject",
            "rejection cites the attempted transition; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn pm_firing_reject_with_reason_succeeds_and_routes_to_ready() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] reject-routes-to-ready",
            "reject if reject_story with reason-cite doesn't route to ready (bundled-default workflow.toml target)",
        )
        .await;
        drive_to_delivered(
            &pm,
            &dev,
            target_id,
            "applications/composition-root/tests/integration_main_modules/acceptance_reject_substrate_enforce_e2e.rs",
        )
        .await;

        let result = pm
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "id": target_id,
                    "reason": "user-job not met — acceptance-walk surfaced X regression"
                }),
            )
            .await
            .expect("reject with reason should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("rejected"),
            "happy-path message cites rejection; got {msg}"
        );
        assert!(
            msg.contains("user-job not met"),
            "audit-trail surfaces the reason verbatim; got {msg}"
        );

        pm.close().await;
        dev.close().await;
    });
}
