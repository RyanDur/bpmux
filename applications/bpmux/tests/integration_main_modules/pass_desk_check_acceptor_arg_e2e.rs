#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn pass_desk_check_missing_acceptor_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pass-desk-check-missing-acceptor",
            "reject if pass_desk_check without acceptor-arg succeeds (illegal-state-representable per Wlaschin)",
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

        let err = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({"number": target_number, "would_file_bug": false}),
            )
            .await
            .expect_err("missing acceptor-arg should be rejected");

        assert!(
            err.contains("\"kind\":\"workflow-gate-mismatch\""),
            "rejection cites structured kind; got {err}"
        );
        assert!(
            err.contains("\"missing_evidence\":[\"acceptor-class\"]"),
            "rejection cites missing acceptor-class evidence; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_invalid_acceptor_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pass-desk-check-invalid-acceptor",
            "reject if pass_desk_check accepts non-canonical acceptor values",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "cite cite cite cite cite cite",
                }),
            )
            .await
            .expect("designer submit_verdict");

        let err = designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "acceptor": "ryan",
                    "would_file_bug": false
                }),
            )
            .await
            .expect_err("invalid acceptor value should be rejected");

        assert!(
            err.contains("\"missing_evidence\":[\"acceptor-class\"]"),
            "rejection cites acceptor-class evidence; got {err}"
        );
        assert!(
            err.contains("\"allowed\":[\"team\",\"owner\",\"dev\"]"),
            "rejection cites allowed enum values; got {err}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_acceptor_team_applies_awaiting_team_label() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Chore] pass-desk-check-team-acceptor",
            "reject if pass_desk_check with acceptor=team does NOT apply awaiting team label atomically",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "src/bar.rs:10 + rg snippet for team-class atomic-label-application",
                }),
            )
            .await
            .expect("designer submit_verdict");

        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "acceptor": "team",
                    "would_file_bug": false
                }),
            )
            .await
            .expect("pass_desk_check with acceptor=team should succeed");

        let story = designer
            .call_tool("get_story", serde_json::json!({"number": target_number}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("awaiting team"),
            "post-pass_desk_check labels include awaiting team; got:\n{story_text}"
        );
        assert!(
            !story_text.contains("awaiting owner"),
            "awaiting owner must NOT coexist with awaiting team (single-acceptor-label invariant); got:\n{story_text}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_acceptor_owner_applies_awaiting_owner_label() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Feature] pass-desk-check-owner-acceptor",
            "reject if pass_desk_check with acceptor=owner does NOT apply awaiting owner label atomically",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "src/baz.rs:99 + rg snippet for owner-class atomic-label-application",
                }),
            )
            .await
            .expect("designer submit_verdict");

        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "acceptor": "owner",
                    "would_file_bug": false
                }),
            )
            .await
            .expect("pass_desk_check with acceptor=owner should succeed");

        let story = designer
            .call_tool("get_story", serde_json::json!({"number": target_number}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("awaiting owner"),
            "post-pass_desk_check labels include awaiting owner; got:\n{story_text}"
        );
        assert!(
            !story_text.contains("awaiting team"),
            "awaiting team must NOT coexist with awaiting owner; got:\n{story_text}"
        );

        pm.close().await;
        designer.close().await;
    });
}

#[test]
fn pass_desk_check_acceptor_dev_applies_awaiting_dev_label() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_number = create_target_story(
            &pm,
            "[Chore] pass-desk-check-dev-acceptor",
            "reject if pass_desk_check with acceptor=dev does NOT apply awaiting dev label atomically",
        )
        .await;

        designer
            .call_tool(
                "submit_verdict",
                serde_json::json!({
                    "story_number": target_number,
                    "verdict": "verified",
                    "substrate_grounding": "src/qux.rs:42 + rg snippet for dev-class atomic-label-application",
                }),
            )
            .await
            .expect("designer submit_verdict");

        designer
            .call_tool(
                "pass_desk_check",
                serde_json::json!({
                    "number": target_number,
                    "acceptor": "dev",
                    "would_file_bug": false
                }),
            )
            .await
            .expect("pass_desk_check with acceptor=dev should succeed");

        let story = designer
            .call_tool("get_story", serde_json::json!({"number": target_number}))
            .await
            .expect("get_story should succeed");
        let story_text = story
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("awaiting dev"),
            "post-pass_desk_check labels include awaiting dev; got:\n{story_text}"
        );
        assert!(
            !story_text.contains("awaiting team"),
            "awaiting team must NOT coexist with awaiting dev (single-acceptor-label invariant); got:\n{story_text}"
        );
        assert!(
            !story_text.contains("awaiting owner"),
            "awaiting owner must NOT coexist with awaiting dev (single-acceptor-label invariant); got:\n{story_text}"
        );

        pm.close().await;
        designer.close().await;
    });
}
