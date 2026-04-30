//! Agent-side E2E test for `deliver_story` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype with verified_observation gate).
//! Uses the existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — deliver_story applicability =
//! {agent: yes, human: no (no human-side deliver affordance —
//! agent-only), external: no}. Single-class agent-only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

const VERIFIED_OBSERVATION: &str = "Drove the new affordance via real bridge → real handler → real /api/board roundtrip; observed expected falsifier outcome.";

#[test]
fn deliver_story_dev_with_verified_observation_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] deliver-target",
            "reject if deliver_story doesn't fire signals + GateCommentAppended",
        )
        .await;

        let result = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({
                    "number": target_number,
                    "verified_observation": VERIFIED_OBSERVATION
                }),
            )
            .await
            .expect("deliver_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn deliver_story_rejects_pm_caller_per_dev_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pm-cannot-deliver-target",
            "reject if pm can deliver bypassing dev-lane gate",
        )
        .await;

        let err = pm
            .call_tool(
                "deliver_story",
                serde_json::json!({
                    "number": target_number,
                    "verified_observation": VERIFIED_OBSERVATION
                }),
            )
            .await
            .expect_err("pm should be rejected by deliver dev-lane gate");

        assert!(
            err.contains("dev") || err.contains("only") || err.contains("restricted"),
            "rejection cites dev-lane discipline; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn deliver_story_rejects_missing_verified_observation() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] missing-verified-observation-target",
            "reject if dev can deliver without articulating verified_observation",
        )
        .await;

        let err = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({"number": target_number}),
            )
            .await
            .expect_err("missing verified_observation should be rejected");

        assert!(
            err.contains("verified_observation"),
            "rejection names the missing field per deliver-gate discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn deliver_story_view_board_reflects_delivered_state_via_fixture_seed() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let body = format!(
        "## Acceptance test path\n\n{}/tests/deliver_story_e2e_test.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    let issue = issue_json(300, "Pre-seeded in-progress", &["in progress"], Some(&body));
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 300, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        dev.call_tool(
            "deliver_story",
            serde_json::json!({
                "number": 300,
                "verified_observation": VERIFIED_OBSERVATION
            }),
        )
        .await
        .expect("deliver_story should succeed against seeded story");

        let story = dev
            .call_tool("get_story", serde_json::json!({"number": 300}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("desk-check"),
            "get_story reflects desk-check label for seeded story #300 post-deliver_story; got:\n{story_text}"
        );
    });
}

#[test]
fn deliver_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = dev
            .call_tool(
                "deliver_story",
                serde_json::json!({"verified_observation": VERIFIED_OBSERVATION}),
            )
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        dev.close().await;
    });
}
