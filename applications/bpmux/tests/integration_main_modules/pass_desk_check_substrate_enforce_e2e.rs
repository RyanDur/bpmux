//! E2E acceptance-test for #1386 — pass_desk_check substrate-
//! enforce via workflow.toml + designer-broadcast evidence-check.
//!
//! Sister of #1385 (accept_story substrate-enforce). Reuses the
//! same workflow-gate primitives: role-check + evidence-check.
//!
//! Four Gherkin scenarios from #1386's body:
//!   1. Designer broadcasts substrate-grounding then fires
//!      pass_desk_check → 200
//!   2. Designer fires without prior substrate-broadcast → 409
//!      workflow-gate-mismatch with missing_evidence
//!   3. Non-reviewer-role fires pass_desk_check → 409
//!      workflow-role-not-allowed
//!   4. Evidence-broadcast verdict-shape regex (sister-substrate
//!      story under #1384; permissive-baseline shape in this slice)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn dev_firing_pass_desk_check_returns_workflow_role_not_allowed() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-pass-desk-check-substrate-refusal",
            "reject if dev firing pass_desk_check isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be substrate-refused");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "rejection cites firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"pass-desk-check\""),
            "rejection cites the attempted transition; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn designer_firing_pass_desk_check_without_broadcast_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-pass-without-evidence",
            "reject if designer firing pass_desk_check without recent substrate-broadcast succeeds",
        )
        .await;

        let err = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("pass_desk_check without designer-evidence should be refused");

        assert!(
            err.contains("\"kind\":\"workflow-gate-mismatch\""),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"designer-verdict-missing\"]"),
            "rejection cites missing designer-verdict evidence; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn designer_firing_pass_desk_check_with_broadcast_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-pass-with-evidence",
            "reject if designer firing pass_desk_check with recent substrate-broadcast doesn't 200",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "rg + git show fragments cited at src/foo.rs:42"
                }),
            )
            .await
            .expect("designer submit_verdict");

        let result = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect("pass_desk_check with designer-evidence should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "happy-path message names the story; got {msg}"
        );

        pm.close().await;
        designer.close().await;
    });
}
