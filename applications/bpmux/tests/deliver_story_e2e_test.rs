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
use std::io::Write;
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
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[{"number":300,"title":"Pre-seeded in-progress","state":"OPEN","labels":[{"name":"in progress"}]}]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect_with_board_fixture(&bpmux, "dev", &fixture_path).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        dev.call_tool(
            "deliver_story",
            serde_json::json!({
                "number": 300,
                "verified_observation": VERIFIED_OBSERVATION
            }),
        )
        .await
        .expect("deliver_story should succeed against seeded story");

        let view = dev
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view.as_str().map(String::from).unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains("#300") && view_text.contains("DELIVERED"),
            "view_board reflects delivered state for seeded story #300 post-deliver_story; got:\n{view_text}"
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
