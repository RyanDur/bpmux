//! Agent-side E2E test for `start_story` MCP tool (#476 Phase 1
//! backfill — state-transition archetype). Uses the existing
//! `tests/harness/bridge.rs` substrate + `--board-fixture` seeding
//! pattern (#481).
//!
//! Per the 3-class framing — start_story applicability =
//! {agent: yes, human: no (no human-side start affordance — agent-only),
//! external: no}. Single-class agent-only coverage.
//!
//! Roundtrip-via-view_board falsifier: closed via `--board-fixture`
//! pre-seeded story (#481 substrate-pattern). The fixture seeds the
//! adapter's InMemorySourceAdapter at bridge startup so dev's bridge
//! has the story in its own adapter (no cross-bridge propagation
//! needed; state IS pre-seeded identically per bridge).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

const PICKUP: &str =
    "When dev picks up this story, dev needs to mark in-progress + paraphrase the user-job.";
const FALSIFIER_REFRAMING: &str =
    "reject if dev silently moves to in-progress without surfacing pickup-paraphrase";

#[test]
fn start_story_dev_returns_in_progress_marker() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] start-target",
            "reject if start_story doesn't transition state to in-progress",
        )
        .await;

        let result = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "number": target_number,
                    "pickup_paraphrase": PICKUP,
                    "falsifier_reframing": FALSIFIER_REFRAMING
                }),
            )
            .await
            .expect("start_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("in progress"),
            "happy-path message names new state; got {msg}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn start_story_rejects_pm_caller_per_dev_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pm-cannot-start-target",
            "reject if pm can call start_story bypassing dev-lane gate",
        )
        .await;

        let err = pm
            .call_tool(
                "start_story",
                serde_json::json!({
                    "number": target_number,
                    "pickup_paraphrase": PICKUP,
                    "falsifier_reframing": FALSIFIER_REFRAMING
                }),
            )
            .await
            .expect_err("pm should be rejected by start_story dev-lane gate");

        assert!(
            err.contains("dev") || err.contains("only") || err.contains("restricted"),
            "rejection cites dev-lane discipline; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn start_story_rejects_missing_pickup_paraphrase() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] missing-pickup-target",
            "reject if dev can start_story without pickup-paraphrase",
        )
        .await;

        let err = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "number": target_number,
                    "falsifier_reframing": FALSIFIER_REFRAMING
                }),
            )
            .await
            .expect_err("missing pickup_paraphrase should be rejected");

        assert!(
            err.contains("pickup_paraphrase"),
            "rejection names the missing field per pickup-gate discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn start_story_view_board_reflects_in_progress_via_board_fixture_seed() {
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let body = format!(
        "## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n",
        env!("CARGO_MANIFEST_DIR")
    );
    let issue = issue_json(200, "Pre-seeded ready story", &["ready"], Some(&body));
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 200, &issue);

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_mock_gh(&bpmux, "dev", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        dev.call_tool(
            "start_story",
            serde_json::json!({
                "number": 200,
                "pickup_paraphrase": PICKUP,
                "falsifier_reframing": FALSIFIER_REFRAMING
            }),
        )
        .await
        .expect("start_story should succeed against seeded story");

        let story = dev
            .call_tool("get_story", serde_json::json!({"number": 200}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("in progress"),
            "get_story reflects in-progress label for seeded story #200 post-start_story; got:\n{story_text}"
        );
    });
}

#[test]
fn start_story_rejects_missing_falsifier_reframing() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] missing-falsifier-target",
            "reject if dev can start_story without falsifier-reframing",
        )
        .await;

        let err = dev
            .call_tool(
                "start_story",
                serde_json::json!({
                    "number": target_number,
                    "pickup_paraphrase": PICKUP
                }),
            )
            .await
            .expect_err("missing falsifier_reframing should be rejected");

        assert!(
            err.contains("falsifier_reframing"),
            "rejection names the missing field per pickup-gate discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}
