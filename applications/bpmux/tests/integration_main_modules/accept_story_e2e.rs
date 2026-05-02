//! Agent-side E2E test for `accept_story` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, multi-role variant). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — accept_story applicability =
//! {agent: yes, human: yes (Electron accept affordance), external: no}.
//! This test covers ONLY agent-side; human-side follows.
//!
//! accept_story allows ryan / pm / designer / dev role-set per current
//! implementation (substrate-gate Feature #479 narrows this in future).
//! Accept also closes the story — view_board reflects Accepted section
//! population.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::io::Write;
use std::time::Duration;

#[test]
fn accept_story_pm_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] accept-target",
            "reject if accept_story doesn't fire signal + state transition + close",
        )
        .await;

        let result = pm
            .call_tool("accept_story", serde_json::json!({"number": target_number}))
            .await
            .expect("accept_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_number}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("accepted") && msg.contains("closed"),
            "happy-path message names accept + close transitions; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn accept_story_designer_can_call() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] designer-accept-target",
            "reject if designer can't call accept_story per multi-role allowlist",
        )
        .await;

        let result = designer
            .call_tool("accept_story", serde_json::json!({"number": target_number}))
            .await
            .expect("designer should be allowed to call accept_story");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains("accepted"),
            "designer-fired accept succeeds; got {msg}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn accept_story_rejects_unknown_role() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let unknown = TestBridge::connect(&bpmux, "intern").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = unknown
            .call_tool("accept_story", serde_json::json!({"number": 100}))
            .await
            .expect_err("unknown role should be rejected");

        assert!(
            err.contains("not a known role") || err.contains("expected one of"),
            "rejection cites the role-vocabulary; got {err}"
        );

        unknown.close().await;
    });
}

#[test]
fn accept_story_view_board_reflects_accepted_via_fixture_seed() {
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            br#"[{"number":600,"title":"Pre-seeded delivered","state":"OPEN","labels":[{"name":"delivered"}]}]"#,
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    // #551: fixture-loading happens server-side via BPMUX_BOARD_FIXTURE
    let bpmux = TestBpmux::start_with_fixture(&fixture_path);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        pm.call_tool(
            "accept_story",
            serde_json::json!({"number": 600}),
        )
        .await
        .expect("accept_story should succeed against seeded story");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view.as_str().map(String::from).unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains("#600") && view_text.contains("ACCEPTED"),
            "view_board reflects accepted state for seeded story #600 post-accept_story; got:\n{view_text}"
        );
    });
}

#[test]
fn accept_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("accept_story", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("number"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}
