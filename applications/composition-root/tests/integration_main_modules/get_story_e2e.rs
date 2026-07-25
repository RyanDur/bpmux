#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use std::time::Duration;

#[test]
fn get_story_returns_full_payload_for_existing_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target_id = create_target_story(
            &pm,
            "[Feature] get-story-target",
            "reject if get_story doesn't return full story payload",
        )
        .await;

        let story = pm
            .call_tool("get_story", serde_json::json!({"id": target_id}))
            .await
            .expect("get_story should succeed");

        assert_eq!(
            story["id"].as_u64(),
            Some(target_id),
            "payload exposes number; got {story:?}"
        );
        assert!(
            story["title"]
                .as_str()
                .map(|t| t.contains("get-story-target"))
                .unwrap_or(false),
            "payload exposes title verbatim; got {story:?}"
        );
        assert!(
            story["workflow_state"].is_string(),
            "payload exposes workflow_state field (renamed from state per #1144); got {story:?}"
        );
        assert!(
            story["is_closed"].is_boolean(),
            "payload exposes is_closed bool field (added per #1144); got {story:?}"
        );
        assert!(
            story.get("state").is_none(),
            "legacy `state` field removed per #1144; only workflow_state + is_closed appear"
        );
        assert!(
            story["body"].is_string(),
            "payload exposes body field; got {story:?}"
        );
        assert!(
            story["labels"].is_array(),
            "payload exposes labels array; got {story:?}"
        );

        pm.close().await;
    });
}

#[test]
fn get_story_rejects_missing_number() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("get_story", serde_json::json!({}))
            .await
            .expect_err("missing number should be rejected");

        assert!(
            err.contains("'id'"),
            "rejection names the missing field; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn get_story_returns_error_for_non_existent_story() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool("get_story", serde_json::json!({"id": 999999}))
            .await
            .expect_err("non-existent story should be rejected");

        assert!(
            err.contains("not found") || err.contains("999999"),
            "rejection names the missing story; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn get_story_body_preserves_user_job_and_falsifier_verbatim() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] get-story-body-roundtrip",
                    "user_job": "USER_JOB_GET_STORY_MARKER",
                    "observation_falsifier": "FALSIFIER_GET_STORY_MARKER"
                }),
            )
            .await
            .expect("create_story should succeed");
        let create_msg = result.as_str().unwrap_or("").to_string();
        let target_id: u64 = create_msg
            [create_msg.find('#').expect("create message names #N") + 1..]
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|s| s.parse().ok())
            .expect("parse story number");

        let story = pm
            .call_tool("get_story", serde_json::json!({"id": target_id}))
            .await
            .expect("get_story should succeed");

        let body = story["body"].as_str().expect("body field");
        assert!(
            body.contains("USER_JOB_GET_STORY_MARKER"),
            "user_job preserved verbatim through create→get round-trip; got body:\n{body}"
        );
        assert!(
            body.contains("FALSIFIER_GET_STORY_MARKER"),
            "observation_falsifier preserved verbatim through create→get round-trip; got body:\n{body}"
        );

        pm.close().await;
    });
}
