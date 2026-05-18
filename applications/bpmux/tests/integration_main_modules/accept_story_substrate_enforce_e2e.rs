//! E2E acceptance-test for #1385 — accept_story substrate-enforce
//! (role-access + evidence-required hard-block at bridge).
//!
//! First vertical-slice of Epic #1384 (configurable workflow substrate).
//! Substrate-enforces the 11 Gherkin scenarios from #1385's body:
//!   1. team-verifiable PM-fire with designer + dev verdicts → 200
//!   2. owner-walk-class without owner-fire-evidence → 409
//!   3. team-verifiable without designer-verdict → 409
//!   4. dev/designer fires coordinator-only-config → 409
//!   5. substrate-tests evidence-broadcast regex against empirical seeds
//!   6. backward-compat — existing accepted stories unaffected
//!   7. workflow.toml missing → bundled-default (graceful greenfield)
//!   8. workflow.toml malformed → 500 + structured error; NEVER silent-fallback
//!   9. concurrent-fire race — second fire returns 409 target-state-invalid
//!  10. self-mutation guard — actor who delivered cannot fire accept
//!  11. workflow.toml configures evidence_window_seconds per transition
//!
//! Slice 2 (role-check): partial coverage of Scenario 4 (dev/designer
//! fires accept on workflow.toml-restricted actor_roles list → 409
//! workflow-role-not-allowed structured error).
//!
//! Slice 3 (evidence-check): coverage of Scenarios 1 + 3 — PM fires
//! team-verifiable accept with both designer + dev verdicts in window
//! → 200; PM fires team-verifiable without designer-verdict → 409
//! workflow-gate-mismatch with missing_evidence cite.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, seed_team_verifiable_evidence};
use std::time::Duration;

#[test]
fn dev_firing_accept_returns_structured_409_with_workflow_role_not_allowed_kind() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-accept-substrate-refusal",
            "reject if dev firing accept_story isn't substrate-refused per workflow.toml actor_roles",
        )
        .await;

        let err = dev
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be substrate-refused");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "structured kind discriminator present; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"dev\""),
            "actor_role names firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"accept\""),
            "attempted_transition names the gate; got {err}"
        );
        assert!(
            err.contains("\"ryan\"") && err.contains("\"pm\""),
            "allowed_roles cites the workflow.toml-configured set; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn pm_fires_team_verifiable_with_both_verdicts_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] accept-with-evidence",
            "reject if PM-fire accept with both designer + dev verdicts in window doesn't 200",
        )
        .await;

        seed_team_verifiable_evidence(&bpmux).await;

        let result = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect("accept_story should succeed with both verdicts present");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("accepted"),
            "happy-path message names accept; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn pm_fires_team_verifiable_without_designer_verdict_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] accept-missing-designer-evidence",
            "reject if PM-fire accept without designer-verdict isn't substrate-refused",
        )
        .await;

        dev.send("verdict: substrate verified").await;

        let err = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("accept_story should be refused without designer-verdict");

        assert!(
            err.contains("\"kind\":\"workflow-gate-mismatch\""),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"class\":\"team-verifiable\""),
            "rejection cites the gate-class; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"designer-verdict\"]"),
            "rejection cites the missing-evidence list; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn pm_fires_team_verifiable_without_dev_verdict_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] accept-missing-dev-evidence",
            "reject if PM-fire accept without dev-verdict isn't substrate-refused",
        )
        .await;

        designer.send("verdict: substrate verified").await;

        let err = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("accept_story should be refused without dev-verdict");

        assert!(
            err.contains("\"missing_evidence\":[\"dev-verdict\"]"),
            "rejection cites dev-verdict missing; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}
