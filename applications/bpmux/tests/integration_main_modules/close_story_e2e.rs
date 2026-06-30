//! Agent-side E2E test for `close_story` MCP tool (#476 Phase 1
//! backfill — gate-fire archetype, terminal-only state). Uses the
//! existing `tests/harness/bridge.rs` substrate.
//!
//! Per the 3-class framing — close_story applicability =
//! {agent: yes, human: yes (Electron close-affordance), external: yes
//! (gh issue close)}. This test covers ONLY agent-side; human-side
//! + external-mutation E2E follow per applicable-class backfill.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn close_story_returns_message_with_audit_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] close-target",
            "reject if close-on-non-existent-story silently succeeds",
        )
        .await;

        let result = pm
            .call_tool(
                "close_story",
                serde_json::json!({
                    "id": target_id,
                    "reason": "superseded by alternate path"
                }),
            )
            .await
            .expect("close_story should succeed");

        let msg = result.as_str().unwrap_or("").to_string();
        assert!(
            msg.contains(&format!("#{target_id}")),
            "message names target story; got {msg}"
        );
        assert!(
            msg.contains("closed"),
            "happy-path message names the gate-fire affordance; got {msg}"
        );
        assert!(
            msg.contains("superseded by alternate path"),
            "message includes the close-reason for audit trail per close discipline; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn close_story_rejects_dev_caller_per_pm_lane_gate() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] dev-cannot-close-target",
            "reject if dev can close a story bypassing pm-lane gate",
        )
        .await;

        let err = dev
            .call_tool(
                "close_story",
                serde_json::json!({
                    "id": target_id,
                    "reason": "dev-attempted-close"
                }),
            )
            .await
            .expect_err("dev should be rejected by close pm-lane gate");

        assert!(
            err.contains("pm") || err.contains("only") || err.contains("restricted"),
            "rejection cites pm-lane discipline; got {err}"
        );

        pm.close().await;
        dev.close().await;
    });
}

#[test]
fn close_story_rejects_empty_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] empty-reason-target",
            "reject if empty-reason close is silently accepted",
        )
        .await;

        let err = pm
            .call_tool(
                "close_story",
                serde_json::json!({
                    "id": target_id,
                    "reason": ""
                }),
            )
            .await
            .expect_err("empty reason should be rejected");

        assert!(
            err.contains("\"missing_evidence\":[\"reason-cite\"]"),
            "structured rejection cites missing reason-cite; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"close\""),
            "rejection cites attempted_transition; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn close_story_view_board_shows_closed_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] close-roundtrip-target",
            "reject if view_board doesn't reflect closed state after close_story",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "id": target_id,
                "reason": "close-roundtrip-marker"
            }),
        )
        .await
        .expect("close_story should succeed");

        let view = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board should succeed");
        let view_text = view
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| view.to_string());
        assert!(
            view_text.contains(&format!("#{target_id}"))
                || view_text.contains("ACCEPTED"),
            "view_board reflects the closed-as-accepted story per is_closed→accepted mapping; got:\n{view_text}"
        );

        pm.close().await;
    });
}

#[test]
fn close_with_disposition_retire_lands_retired_regardless_of_reason_phrasing() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] disposition-retire-target",
            "reject if disposition=retire with a non-keyword reason mis-files as accepted",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "id": target_id,
                "reason": "Split into #9999",
                "disposition": "retire"
            }),
        )
        .await
        .expect("close_story with disposition=retire should succeed");

        let story = pm
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        assert_eq!(
            story["workflow_state"].as_str(),
            Some("retired"),
            "disposition=retire must land the story Retired regardless of reason phrasing — the reason carried NO retire-keyword, so the old is_retire_reason string-match would mis-file it accepted; got {story}"
        );

        pm.close().await;
    });
}

#[test]
fn close_with_disposition_accept_does_not_retire_even_with_retire_keyword_reason() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] disposition-accept-target",
            "reject if a retire-keyword reason forces retire when disposition=accept",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "id": target_id,
                "reason": "superseded wording but accept disposition",
                "disposition": "accept"
            }),
        )
        .await
        .expect("close_story with disposition=accept should succeed");

        let story = pm
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        assert_ne!(
            story["workflow_state"].as_str(),
            Some("retired"),
            "disposition=accept must NOT retire even when the reason contains a retire-keyword — disposition is the determinant, not the reason string; got {story}"
        );

        pm.close().await;
    });
}

#[test]
fn close_without_disposition_defaults_to_retire() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] disposition-default-target",
            "reject if an omitted disposition does not default to retire",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "id": target_id,
                "reason": "closing this mid-pipeline filing"
            }),
        )
        .await
        .expect("close_story without an explicit disposition should succeed");

        let story = pm
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        assert_eq!(
            story["workflow_state"].as_str(),
            Some("retired"),
            "an OMITTED disposition must DEFAULT to retire — close_story is the retire-class tool; if the default regresses to accept, an omitted-disposition supersede silently mis-files as accepted (the bug this fixes); got {story}"
        );

        pm.close().await;
    });
}

#[test]
fn close_with_disposition_wontfix_lands_retired() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] disposition-wontfix-target",
            "reject if disposition=wontfix does not land Retired",
        )
        .await;

        pm.call_tool(
            "close_story",
            serde_json::json!({
                "id": target_id,
                "reason": "out of scope for this milestone",
                "disposition": "wontfix"
            }),
        )
        .await
        .expect("close_story with disposition=wontfix should succeed");

        let story = pm
            .call_tool("get_story", serde_json::json!({ "id": target_id }))
            .await
            .expect("get_story should succeed");

        assert_eq!(
            story["workflow_state"].as_str(),
            Some("retired"),
            "disposition=wontfix lands Retired (closed && !accepted) — won't-fix is not shipped; got {story}"
        );

        pm.close().await;
    });
}

#[test]
fn close_story_does_not_fire_post_mutation_get_issue_round_trip() {
    // #1151: close_story handler used to follow its mutate_sync with
    // r.refresh_story(N).await — a redundant GET round-trip to GH
    // for state the close-mutation response already carried. The
    // tracker-response capture cascade (StoryRefreshed reducer applies
    // the post-close Issue) makes the follow-up refresh redundant;
    // this test pins that the redundant GET is gone.
    use harness::mock_gh::{issue_json, mock_story_mutations};
    use httpmock::{Method, MockServer};
    use serde_json::Value;

    let server = MockServer::start();
    let base_url = format!("{}/", server.base_url());
    let issue = issue_json(900, "Pre-seeded delivered", &["delivered"], None);
    server.mock(|when, then| {
        when.method(Method::GET).path("/repos/acme/bpmux/issues");
        then.status(200)
            .header("content-type", "application/json")
            .body(Value::Array(vec![issue.clone()]).to_string());
    });
    mock_story_mutations(&server, 900, &issue);

    // Track GET /repos/acme/bpmux/issues/900 separately so we can
    // count post-mutation hits — `mock_story_mutations` registered a
    // GET at this path; we add an additional mock that captures hits
    // and asserts a stable upper bound. httpmock matches the
    // most-recently-registered mock first when paths overlap; we
    // register this one AFTER the helper so it captures all hits.
    let issue_detail_get = server.mock(|when, then| {
        when.method(Method::GET)
            .path("/repos/acme/bpmux/issues/900");
        then.status(200)
            .header("content-type", "application/json")
            .body(issue.to_string());
    });

    let bpmux = TestBpmux::start_with_mock_gh(&base_url);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect_with_mock_gh(&bpmux, "pm", &base_url).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Capture pre-call hit count so we measure only the
        // close_story-initiated GETs, not the prior bring-up traffic.
        let before = issue_detail_get.calls();

        pm.call_tool(
            "close_story",
            serde_json::json!({"id": 900, "reason": "post-mutation-refresh-test"}),
        )
        .await
        .expect("close_story should succeed against seeded story");

        let after = issue_detail_get.calls();
        let close_story_gets = after - before;

        // Pre-#1151 fix: handle_close_story fired r.refresh_story(N)
        // post-mutation, generating exactly one GET /repos/.../issues/900
        // round-trip. Post-fix: zero — the StoryRefreshed reducer
        // applies the post-close Issue captured from the close-mutation
        // response. Allow a small upper bound (≤1) to tolerate any
        // refresh fired by adjacent handlers (label_story uses a
        // similar pattern, but we don't trigger that here); strictly
        // assert that we're below the pre-fix baseline of 2+ GETs
        // (one pre-write read + one post-write refresh).
        assert!(
            close_story_gets <= 1,
            "close_story handler must not fire a redundant post-mutation \
             GET /repos/.../issues/900; saw {close_story_gets} hits this call"
        );
    });
}
