#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::get_story::handle_get_story;
use crate::bridge::tracker::{StoryTracker, WorkflowRunEntry};

#[derive(Clone)]
struct ReworkTracker {
    story: serde_json::Value,
    comments: Vec<serde_json::Value>,
}

impl StoryTracker for ReworkTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        Ok(format!("#{number} closed"))
    }
    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} labeled '{label}'"))
    }
    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} unlabeled '{label}'"))
    }
    async fn create_story(&self, title: &str, _body: &str) -> Result<String, String> {
        Ok(format!("created: {title}"))
    }
    async fn list_stories(&self) -> Result<String, String> {
        Ok("[]".to_string())
    }
    async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        Ok(Vec::new())
    }
    async fn get_story(&self, _number: u64) -> Result<Option<serde_json::Value>, String> {
        Ok(Some(self.story.clone()))
    }
    async fn comment_on_story(&self, _number: u64, _body: &str) -> Result<String, String> {
        Ok("commented".to_string())
    }
    async fn list_story_comments(&self, _number: u64) -> Result<Vec<serde_json::Value>, String> {
        Ok(self.comments.clone())
    }
    async fn get_story_title(&self, number: u64) -> Option<String> {
        Some(format!("story #{number}"))
    }
    async fn get_story_labels(&self, _number: u64) -> Vec<String> {
        Vec::new()
    }
    async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
        Ok(None)
    }
}

#[tokio::test]
async fn enriches_body_for_rejected_story() {
    let tracker = ReworkTracker {
        story: serde_json::json!({
            "number": 443,
            "title": "Input ref rework",
            "body": "original story body text",
            "state": "open",
            "labels": [{"name": "rejected"}],
        }),
        comments: vec![
            serde_json::json!({
                "body": "**verified_observation** (dev deliver-gate):\n\nprior delivery ran end-to-end",
                "created_at": "2026-04-23T00:00:00Z",
            }),
            serde_json::json!({
                "body": "**rejection_reason** (reject-gate by ryan):\n\nchannelKeyRef is not legitimate",
                "created_at": "2026-04-23T01:00:00Z",
            }),
        ],
    };
    let args = serde_json::json!({"number": 443});
    let result = handle_get_story(args.as_object().unwrap(), &tracker, None, None)
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.starts_with("## Rework context"),
        "body should open with rework context: {body}"
    );
    assert!(
        body.contains("channelKeyRef is not legitimate"),
        "body should surface rejection reason: {body}"
    );
    assert!(
        body.contains("prior delivery ran end-to-end"),
        "body should surface verified_observation: {body}"
    );
    assert!(
        body.ends_with("original story body text"),
        "original body should be preserved at end: {body}"
    );
}

#[tokio::test]
async fn enriches_body_for_needs_fixes_story() {
    let tracker = ReworkTracker {
        story: serde_json::json!({
            "number": 200,
            "title": "Foo feature",
            "body": "original body",
            "state": "open",
            "labels": [{"name": "needs fixes"}],
        }),
        comments: vec![serde_json::json!({
            "body": "**rejection_reason** (desk-check-fail by designer):\n\nfailed spec",
            "created_at": "2026-04-23T01:00:00Z",
        })],
    };
    let args = serde_json::json!({"number": 200});
    let result = handle_get_story(args.as_object().unwrap(), &tracker, None, None)
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(body.contains("## Rework context"));
    assert!(body.contains("failed spec"));
    assert!(body.contains("desk-check-fail by designer"));
}

#[tokio::test]
async fn does_not_enrich_body_for_non_rework_story() {
    let tracker = ReworkTracker {
        story: serde_json::json!({
            "number": 100,
            "title": "Fresh feature",
            "body": "original body",
            "state": "open",
            "labels": [{"name": "ready"}],
        }),
        comments: vec![serde_json::json!({
            "body": "**rejection_reason** (reject-gate by ryan):\n\nstale reject",
        })],
    };
    let args = serde_json::json!({"number": 100});
    let result = handle_get_story(args.as_object().unwrap(), &tracker, None, None)
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(
        body, "original body",
        "non-rework stories should not be enriched: {body}"
    );
}

#[tokio::test]
async fn skips_enrichment_when_no_markers_in_comments() {
    let tracker = ReworkTracker {
        story: serde_json::json!({
            "number": 50,
            "title": "Reworked item",
            "body": "plain body",
            "state": "open",
            "labels": [{"name": "rejected"}],
        }),
        comments: vec![serde_json::json!({"body": "just a casual comment"})],
    };
    let args = serde_json::json!({"number": 50});
    let result = handle_get_story(args.as_object().unwrap(), &tracker, None, None)
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "plain body");
}
