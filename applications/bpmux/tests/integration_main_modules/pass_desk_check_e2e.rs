//! Agent-side E2E test for `pass_desk_check` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, designer-only). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — pass_desk_check applicability =
//! {agent: yes, human: no, external: no}. Single-class agent-only.
//!
//! Mirrors `fail_desk_check_e2e_test.rs` but transitions Delivered →
//! ReadyForAccept (vs. NeedsFixes) and requires no `reason` field on
//! the success path.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn pass_desk_check_designer_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pass-desk-check-target",
            "reject if pass_desk_check doesn't fire signal + state transition",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": target_number,
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
            .expect("pass_desk_check should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("passed desk-check"),
            "happy-path message names the gate-fire affordance; got {msg}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_rejects_dev_caller_per_designer_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] dev-cannot-pass-desk-check-target",
            "reject if dev can pass desk-check bypassing designer-lane gate",
        )
        .await;

        let err = dev
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect_err("dev should be rejected by pass_desk_check designer-lane gate");

        assert!(
            err.contains("designer") || err.contains("only") || err.contains("restricted"),
            "rejection cites designer-lane discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn pass_desk_check_view_board_reflects_ready_for_accept_via_fixture_seed() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(500, "Pre-seeded delivered", &["delivered"], None);
    mock_story_full(&server, 500, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer =
            TestBridge::connect_with_mock_gh(&bpmux, "designer", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": 500,
                    "verdict": "verified",
                    "substrate_grounding": "rg + git show fragments cited at src/foo.rs:42"
                }),
            )
            .await
            .expect("designer submit_verdict");

        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": 500, "acceptor": "owner", "would_file_bug": false}),
            )
            .await
            .expect("pass_desk_check should succeed against seeded story");

        let story = designer
            .call_tool("get_story", serde_json::json!({"number": 500}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("awaiting owner"),
            "get_story reflects the awaiting-owner acceptor-label that pass_desk_check(acceptor=owner) applies — the ready-for-accept signal the test name promises; a faithful list-mock is required since a stale-list clobber would strip it; got:\n{story_text}"
        );
        assert!(
            story_text.contains("delivered"),
            "pass_desk_check keeps the delivered label (reap_workflow_labels keeps LABEL_DELIVERED; state stays Delivered) for seeded story #500; got:\n{story_text}"
        );
    });
}

#[test]
fn pass_desk_check_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = designer
            .call_tool("pass_desk_check", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        designer.close().await;
    });
}
