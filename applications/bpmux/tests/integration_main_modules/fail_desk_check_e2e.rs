//! Agent-side E2E test for `fail_desk_check` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, designer-only). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — fail_desk_check applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn fail_desk_check_designer_with_reason_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] fail-desk-check-target",
            "reject if fail_desk_check doesn't fire signal + state transition",
        )
        .await;

        let result = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "reason": "Component fails accessibility-baseline scan"
                }),
            )
            .await
            .expect("fail_desk_check should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("failed desk-check"),
            "happy-path message names the gate-fire affordance; got {msg}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn fail_desk_check_rejects_dev_caller_per_designer_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-cannot-fail-desk-check-target",
            "reject if dev can fail desk-check bypassing designer-lane gate",
        )
        .await;

        let err = dev
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "reason": "dev attempted desk-check fail"
                }),
            )
            .await
            .expect_err("dev should be rejected by fail_desk_check designer-lane gate");

        assert!(
            err.contains("designer") || err.contains("only") || err.contains("restricted"),
            "rejection cites designer-lane discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn fail_desk_check_rejects_missing_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] missing-reason-target",
            "reject if designer can fail desk-check without articulating reason",
        )
        .await;

        let err = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({"number": target_number}),
            )
            .await
            .expect_err("missing reason should be rejected");

        assert!(
            err.contains("reason"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn fail_desk_check_view_board_reflects_in_progress_via_fixture_seed() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(400, "Pre-seeded delivered", &["delivered"], None);
    mock_story_full(&server, 400, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer =
            TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({
                    "number": 400,
                    "reason": "test rework needed"
                }),
            )
            .await
            .expect("fail_desk_check should succeed against seeded story");

        let story = designer
            .call_tool("get_story", serde_json::json!({"number": 400}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        // #1146 slice 1: fail_desk_check transitions DeskCheck → InProgress
        // (back-edge); rework-context lives in transition history (gate-
        // comment log), not a "needs fixes" label.
        assert!(
            story_text.contains("in progress"),
            "get_story reflects in-progress label for seeded story #400 post-fail_desk_check; got:\n{story_text}"
        );
    });
}

#[test]
fn fail_desk_check_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = designer
            .call_tool(
                "fail_desk_check",
                serde_json::json!({"reason": "any reason"}),
            )
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        designer.close().await;
    });
}
