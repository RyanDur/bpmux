#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::board::domain::story_type_from_labels_or_title;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn start_story_rejects_missing_pickup_paraphrase() {
    let args = serde_json::json!({
        "number": 398,
        "falsifier_reframing": "I would reject if the pickup signal is invisible."
    });
    let result = handle_start_story(args.as_object().unwrap(), "dev:claude-code", None).await;
    assert!(
        result.is_err(),
        "missing pickup_paraphrase must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("pickup_paraphrase"),
        "error should name the field: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("pickup time"),
        "error should cite retro framing so agents know the why: {err}"
    );
}

#[tokio::test]
async fn start_story_rejects_empty_pickup_paraphrase() {
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "   \n  ",
        "falsifier_reframing": "I would reject if..."
    });
    let result = handle_start_story(args.as_object().unwrap(), "dev:claude-code", None).await;
    assert!(
        result.is_err(),
        "whitespace-only pickup_paraphrase must fail"
    );
    assert!(result.unwrap_err().contains("pickup_paraphrase"));
}

#[tokio::test]
async fn start_story_rejects_missing_falsifier_reframing() {
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "As a dev, I restate the user-job..."
    });
    let result = handle_start_story(args.as_object().unwrap(), "dev:claude-code", None).await;
    assert!(
        result.is_err(),
        "missing falsifier_reframing must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("falsifier_reframing"),
        "error should name the field: {err}"
    );
}

#[tokio::test]
async fn start_story_validation_passes_then_refresh_required_after_5n() {
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "User-job: dev articulates their read of the story at pickup so the team knows work is in flight.",
        "falsifier_reframing": "I would reject if start_story without the new fields still succeeds."
    });
    let result = handle_start_story(args.as_object().unwrap(), "dev:claude-code", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn start_story_still_dev_only_with_new_params() {
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "any text",
        "falsifier_reframing": "any text"
    });
    for caller in ["pm:claude-code", "designer:claude-code"] {
        let result = handle_start_story(args.as_object().unwrap(), caller, None).await;
        assert!(
            result.is_err(),
            "{caller} (non-dev, non-owner) must be rejected by workflow-substrate role-gate"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "{caller} rejection should cite structured kind; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"pickup\""),
            "{caller} rejection should cite attempted_transition; got {err}"
        );
    }
}
