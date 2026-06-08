//! Agent-side E2E test for `reject_story` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, multi-role variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — reject_story applicability =
//! {agent: yes, human: yes (Electron reject affordance), external: no}.
//! This test covers ONLY agent-side; human-side follows.
//!
//! reject_story allows ryan / pm / designer / dev role-set per current
//! implementation (substrate-gate Feature #479 narrows this in future).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn reject_story_pm_with_reason_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] reject-target",
            "reject if reject_story doesn't fire signal + state transition",
        )
        .await;

        let result = pm
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "user-story not satisfied; rework needed"
                }),
            )
            .await
            .expect("reject_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("rejected"),
            "happy-path message names the gate-fire affordance; got {msg}"
        );
        assert!(
            msg.contains("user-story not satisfied"),
            "message includes the rejection reason for audit trail; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn reject_story_designer_refused_per_workflow_substrate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-reject-target",
            "reject if designer firing reject_story isn't substrate-refused per #1389 workflow.toml actor_roles",
        )
        .await;

        let err = designer
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "designer-flagged accessibility violation"
                }),
            )
            .await
            .expect_err("designer should be substrate-refused per workflow.toml actor_roles");

        assert!(
            err.contains("workflow-role-not-allowed"),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"actor_role\":\"designer\""),
            "rejection cites the firing role; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"reject\""),
            "rejection cites the attempted transition; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn reject_story_rejects_unknown_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let unknown = TestBridge::connect(&bpmux, "intern").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = unknown
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "number": 100,
                    "reason": "intern attempted reject"
                }),
            )
            .await
            .expect_err("unknown role should be rejected");

        assert!(
            err.contains("restricted") || err.contains("expected one of") || err.contains("only"),
            "rejection cites allowed roles; got {err}"
        );

        unknown.close().await;
    });
}

#[test]
fn reject_story_view_board_reflects_ready_via_fixture_seed() {
    use harness::mock_gh::{issue_json, mock_story_full};
    use httpmock::MockServer;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(700, "Pre-seeded delivered", &["delivered"], None);
    mock_story_full(&server, 700, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        pm.call_tool(
            "reject_story",
            serde_json::json!({"number": 700, "reason": "user-story not satisfied"}),
        )
        .await
        .expect("reject_story should succeed against seeded story");

        let story = pm
            .call_tool("get_story", serde_json::json!({"number": 700}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        // #1146 slice 1: reject_story transitions Delivered → Ready
        // (back-edge); rework-context lives in transition history (gate-
        // comment log), not a "rejected" label.
        assert!(
            story_text.contains("ready"),
            "get_story reflects ready label for seeded story #700 post-reject_story; got:\n{story_text}"
        );
    });
}

#[test]
fn reject_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("reject_story", serde_json::json!({"reason": "any"}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
