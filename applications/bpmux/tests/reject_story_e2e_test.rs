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

mod harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::io::Write;
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
fn reject_story_designer_can_call() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-reject-target",
            "reject if designer can't call reject_story per multi-role allowlist",
        )
        .await;

        let result = designer
            .call_tool(
                "reject_story",
                serde_json::json!({
                    "number": target_number,
                    "reason": "designer-flagged accessibility violation"
                }),
            )
            .await
            .expect("designer should be allowed to call reject_story");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("rejected"),
            "designer-fired reject succeeds; got {msg}"
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
            err.contains("restricted") || err.contains("only"),
            "rejection cites allowed roles; got {err}"
        );

        unknown.close().await;
    });
}

#[test]
fn reject_story_view_board_reflects_rejected_via_fixture_seed() {
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[{"number":700,"title":"Pre-seeded delivered","state":"OPEN","labels":[{"name":"delivered"}]}]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_board_fixture(&bpmux, "pm", &fixture_path).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        pm.call_tool(
            "reject_story",
            serde_json::json!({"number": 700, "reason": "user-story not satisfied"}),
        )
        .await
        .expect("reject_story should succeed against seeded story");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains("#700") && view_text.contains("REJECTED"),
            "view_board reflects rejected state for seeded story #700 post-reject_story; got:\n{view_text}"
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
