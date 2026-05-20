#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::{
    connected_ws_sink_for_test, connected_ws_sink_with_capture, test_ci_state_store_green,
};
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn create_story_rejects_non_pm_role() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "dev-filed story",
        "user_job": "as dev, I want X",
        "observation_falsifier": "reject if X"
    });
    for role in ["dev", "designer", "ryan", "unknown"].iter() {
        let result = handle_create_story(args.as_object().unwrap(), role, &ws_tx, None).await;
        assert!(
            result.is_err(),
            "create_story should reject {role} calls; got: {result:?}"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("pm-lane"),
            "error should name pm-lane. role={role}, err={err}"
        );
        assert!(
            err.contains("broadcast"),
            "error should redirect to broadcast. role={role}, err={err}"
        );
    }
}

#[tokio::test]
async fn create_story_pm_role_passes_role_gate_then_hits_refresh_required() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "pm-filed story",
        "user_job": "as pm, I want X",
        "observation_falsifier": "reject if X"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn create_story_pm_with_tool_suffix_role_gate_passes_then_refresh_required() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "pm-filed story",
        "user_job": "as pm, I want X",
        "observation_falsifier": "reject if X"
    });
    let result =
        handle_create_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn create_story_rejects_missing_user_job() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "body": "body text",
        "observation_falsifier": "I would reject if X"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "missing user_job should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("user_job"),
        "error should name the field. got: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("user-job"),
        "error should cite the retro framing. got: {err}"
    );
}

#[tokio::test]
async fn create_story_rejects_empty_user_job() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "user_job": "   ",
        "observation_falsifier": "I would reject if X"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "whitespace-only user_job should fail");
    assert!(result.unwrap_err().contains("user_job"));
}

#[tokio::test]
async fn create_story_rejects_missing_observation_falsifier() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "user_job": "When I X, I need Y because Z"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "missing observation_falsifier should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("observation_falsifier"),
        "error should name the field. got: {err}"
    );
}

#[tokio::test]
async fn create_story_validation_passes_then_refresh_required_after_5n() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "body": "author body",
        "user_job": "When I X, I need Y because Z",
        "observation_falsifier": "I would reject if I observed W"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn deliver_story_rejects_missing_verified_observation() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 391
    });
    let result =
        handle_deliver_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "missing verified_observation should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("verified_observation"),
        "error should name the field. got: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("drove"),
        "error should cite retro framing / drove-end-to-end. got: {err}"
    );
}

#[tokio::test]
async fn deliver_story_rejects_empty_verified_observation() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 391,
        "verified_observation": ""
    });
    let result =
        handle_deliver_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "empty verified_observation should fail");
    assert!(result.unwrap_err().contains("verified_observation"));
}

#[tokio::test]
async fn label_story_default_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_add_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "blocked",
        "operation": "add"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_remove_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "in progress",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_remove_idempotent_call_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 99,
        "label": "never-attached-label",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_rejects_invalid_operation_in_migrated_path() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        42,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "nuke"
    });
    let result = handle_label_story(
        args.as_object().unwrap(),
        "pm:claude-code",
        &ws_tx,
        Some(&orchestrator),
    )
    .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("operation"));
    assert!(err.contains("add") && err.contains("remove"));
}

#[tokio::test]
async fn label_story_add_propagates_tracker_error_without_broadcast() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "add"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "add-with-API-error must surface error: {result:?}"
    );
}

#[tokio::test]
async fn label_story_remove_propagates_tracker_error_without_broadcast() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "remove-with-API-error must surface error: {result:?}"
    );
}

#[tokio::test]
async fn label_story_still_pm_gated_with_operation_param() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "remove"
    });
    for caller in ["dev:claude-code", "designer:claude-code", "ryan:human"] {
        let result = handle_label_story(args.as_object().unwrap(), caller, &ws_tx, None).await;
        assert!(result.is_err(), "{caller} must be rejected by PM-only gate");
        assert!(
            result.unwrap_err().contains("restricted to pm"),
            "{caller} rejection should cite role restriction"
        );
    }
}
