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

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::io::Write;
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

        let result = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number}),
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
                serde_json::json!({"number": target_number}),
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
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[{"number":500,"title":"Pre-seeded delivered","state":"OPEN","labels":[{"name":"delivered"}]}]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let designer = TestBridge::connect_with_board_fixture(&bpmux, "designer", &fixture_path).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": 500}),
            )
            .await
            .expect("pass_desk_check should succeed against seeded story");

        let view = designer
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view.as_str().map(String::from).unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains("#500") && view_text.contains("READY FOR ACCEPT"),
            "view_board reflects ready-for-accept state for seeded story #500 post-pass_desk_check; got:\n{view_text}"
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
