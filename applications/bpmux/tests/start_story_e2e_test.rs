//! Agent-side E2E test for `start_story` MCP tool (#476 Phase 1
//! backfill — state-transition archetype). Uses the existing
//! `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — start_story applicability =
//! {agent: yes, human: no (no human-side start affordance — agent-only),
//! external: no}. Single-class agent-only coverage.
//!
//! **Deferred falsifier — view_board roundtrip-coherence**: each
//! `TestBridge` subprocess has its own RefreshDriver+BoardStore (the
//! `--fake-tracker` InMemorySourceAdapter is per-bridge). Stories
//! created on pm's bridge are not visible on dev's bridge without
//! real-GH propagation. For affordances split across roles (pm
//! creates, dev starts), the cross-bridge state-share gap blocks
//! the view_board roundtrip-coherence falsifier. Substrate options
//! (board-fixture seed / shared-adapter fixture / per-bridge intra-role
//! roundtrips only) deferred per
//! `feedback_yagni_with_premise_discipline` until a shape-decision lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

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
