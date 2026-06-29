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

        pm.call_tool(
            "label_story",
            serde_json::json!({
                "number": target_number,
                "label": "awaiting team",
                "rationale": "test fixture: acceptor=team to assert allowed_roles=[pm] (owner bypasses via #1548 superset)"
            }),
        )
        .await
        .expect("pm label_story awaiting team");

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
            err.contains("\"pm\"") && !err.contains("\"owner\""),
            "allowed_roles cites the team set [pm]; the owner is absent because it bypasses via superset (#1548); got {err}"
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

        seed_team_verifiable_evidence(&bpmux, target_number).await;

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
fn dev_accept_story_succeeds_on_awaiting_dev_chore_with_designer_evidence() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Chore] dev-accepts-chore-with-designer-evidence",
            "reject if dev-fire accept on an awaiting-dev Chore with designer-verdict doesn't 200 (chore-ownership-triad accept-half)",
        )
        .await;

        pm.call_tool(
            "label_story",
            serde_json::json!({
                "number": target_number,
                "label": "awaiting dev",
                "rationale": "test fixture: acceptor=dev (Chore dev-acceptor)"
            }),
        )
        .await
        .expect("pm label_story awaiting dev");

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "test-harness: designer verdict = required-evidence for dev-verifiable accept"
                }),
            )
            .await
            .expect("designer submit_verdict");

        let result = dev
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect("dev accept_story should succeed on awaiting-dev Chore with designer-evidence");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("accepted"),
            "dev-accept happy-path message names accept; got {msg}"
        );

        pm.close().await;
        dev.close().await;
        designer.close().await;
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

        pm.call_tool(
            "label_story",
            serde_json::json!({
                "number": target_number,
                "label": "awaiting team",
                "rationale": "test fixture: acceptor=team"
            }),
        )
        .await
        .expect("pm label_story awaiting team");

        dev.call_tool(
            "submit_verdict",
            serde_json::json!({
                "story_id": target_number,
                "verdict": "verified",
                "substrate_grounding": "test-harness: dev verdict (no designer pair)"
            }),
        )
        .await
        .expect("dev submit_verdict");

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
            err.contains("\"missing_evidence\":[\"designer-verdict-missing\"]"),
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

        pm.call_tool(
            "label_story",
            serde_json::json!({
                "number": target_number,
                "label": "awaiting team",
                "rationale": "test fixture: acceptor=team"
            }),
        )
        .await
        .expect("pm label_story awaiting team");

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_id": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "test-harness: designer verdict (no dev pair)"
                }),
            )
            .await
            .expect("designer submit_verdict");

        let err = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("accept_story should be refused without dev-verdict");

        assert!(
            err.contains("\"missing_evidence\":[\"dev-verdict-missing\"]"),
            "rejection cites dev-verdict missing; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pm_firing_accept_on_owner_walk_story_returns_role_not_allowed() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] owner-walk-target",
            "reject if PM firing accept on a gate:owner-walk story doesn't return role-not-allowed",
        )
        .await;

        pm.call_tool(
            "label_story",
            serde_json::json!({"number": target_number, "label": "gate:owner-walk"}),
        )
        .await
        .expect("label_story to add gate:owner-walk");

        let err = pm
            .call_tool(
                "accept_story",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("pm firing owner-walk accept should be refused");

        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"pm\""),
            "rejection cites pm as firing role; got {err}"
        );
        assert!(
            err.contains("\"allowed_roles\":[]"),
            "owner-walk lists no team actor; the owner bypasses via the #1548 superset; got {err}"
        );

        pm.close().await;
    });
}
