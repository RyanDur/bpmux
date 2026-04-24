#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_story_timeline::{build_timeline, handle_view_story_timeline};
use crate::bridge::tracker::{StoryTracker, WorkflowRunEntry};

#[test]
fn filed_event_sourced_from_story_metadata() {
    let story = serde_json::json!({
        "number": 1,
        "created_at": "2026-04-20T00:00:00Z",
        "user": {"login": "ryan"},
    });
    let events = build_timeline(&story, &[]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[0]["ts"], "2026-04-20T00:00:00Z");
    assert_eq!(events[0]["actor"], "ryan");
}

#[test]
fn filed_event_actor_falls_back_to_unknown_when_missing_user() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let events = build_timeline(&story, &[]);
    assert_eq!(events[0]["actor"], "unknown");
}

#[test]
fn filed_event_absent_when_story_has_no_created_at() {
    let story = serde_json::json!({"number": 1});
    let events = build_timeline(&story, &[]);
    assert!(events.is_empty());
}

#[test]
fn pickup_comment_becomes_pickup_event() {
    let story =
        serde_json::json!({"created_at": "2026-04-20T00:00:00Z", "user": {"login": "ryan"}});
    let comments = vec![serde_json::json!({
        "body": "**pickup_paraphrase** (dev pickup-gate):\n\nrestatement here",
        "created_at": "2026-04-21T00:00:00Z",
        "author": "dev:claude-code",
    })];
    let events = build_timeline(&story, &comments);
    let pickup = events
        .iter()
        .find(|e| e["gate"] == "pickup")
        .expect("pickup event");
    assert_eq!(pickup["ts"], "2026-04-21T00:00:00Z");
    assert_eq!(pickup["actor"], "dev:claude-code");
    assert!(
        pickup["payload"]
            .as_str()
            .unwrap()
            .contains("restatement here")
    );
}

#[test]
fn delivered_comment_becomes_delivered_event() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "**verified_observation** (dev deliver-gate):\n\nran end-to-end",
        "created_at": "2026-04-22T00:00:00Z",
        "author": "dev",
    })];
    let events = build_timeline(&story, &comments);
    assert_eq!(
        events.iter().find(|e| e["gate"] == "delivered").unwrap()["ts"],
        "2026-04-22T00:00:00Z"
    );
}

#[test]
fn rejection_comment_classified_as_rejected_by_default() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "**rejection_reason** (reject-gate by ryan):\n\nchannelKeyRef not allowed",
        "created_at": "2026-04-23T00:00:00Z",
        "author": "ryan",
    })];
    let events = build_timeline(&story, &comments);
    assert!(events.iter().any(|e| e["gate"] == "rejected"));
    assert!(!events.iter().any(|e| e["gate"] == "fail_desk_check"));
}

#[test]
fn rejection_comment_with_desk_check_fail_tag_classified_as_fail_desk_check() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "**rejection_reason** (desk-check-fail by designer):\n\nspec failed",
        "created_at": "2026-04-23T00:00:00Z",
        "author": "designer",
    })];
    let events = build_timeline(&story, &comments);
    assert!(events.iter().any(|e| e["gate"] == "fail_desk_check"));
    assert!(!events.iter().any(|e| e["gate"] == "rejected"));
}

#[test]
fn pass_desk_check_comment_becomes_pass_desk_check_event() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "**pass_desk_check** (desk-check-pass by designer)",
        "created_at": "2026-04-24T00:00:00Z",
        "author": "designer",
    })];
    let events = build_timeline(&story, &comments);
    assert!(events.iter().any(|e| e["gate"] == "pass_desk_check"));
}

#[test]
fn accepted_comment_becomes_accepted_event() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "**accepted** (accept-gate by ryan)",
        "created_at": "2026-04-25T00:00:00Z",
        "author": "ryan",
    })];
    let events = build_timeline(&story, &comments);
    assert!(events.iter().any(|e| e["gate"] == "accepted"));
}

#[test]
fn unmarked_comments_are_ignored() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![serde_json::json!({
        "body": "just a chat aside",
        "created_at": "2026-04-21T00:00:00Z",
        "author": "pm",
    })];
    let events = build_timeline(&story, &comments);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["gate"], "filed");
}

#[test]
fn events_sorted_chronologically() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![
        serde_json::json!({
            "body": "**accepted** (accept-gate by ryan)",
            "created_at": "2026-04-25T00:00:00Z",
            "author": "ryan",
        }),
        serde_json::json!({
            "body": "**pickup_paraphrase** (dev pickup-gate):\n\nx",
            "created_at": "2026-04-21T00:00:00Z",
            "author": "dev",
        }),
        serde_json::json!({
            "body": "**verified_observation** (dev deliver-gate):\n\ny",
            "created_at": "2026-04-23T00:00:00Z",
            "author": "dev",
        }),
    ];
    let events = build_timeline(&story, &comments);
    let gates: Vec<&str> = events.iter().map(|e| e["gate"].as_str().unwrap()).collect();
    assert_eq!(gates, vec!["filed", "pickup", "delivered", "accepted"]);
}

#[test]
fn rework_cycle_renders_both_rejection_and_subsequent_pickup() {
    let story = serde_json::json!({"created_at": "2026-04-20T00:00:00Z"});
    let comments = vec![
        serde_json::json!({
            "body": "**pickup_paraphrase** (dev pickup-gate):\n\nfirst pickup",
            "created_at": "2026-04-21T00:00:00Z",
            "author": "dev",
        }),
        serde_json::json!({
            "body": "**verified_observation** (dev deliver-gate):\n\nfirst delivery",
            "created_at": "2026-04-22T00:00:00Z",
            "author": "dev",
        }),
        serde_json::json!({
            "body": "**rejection_reason** (reject-gate by ryan):\n\nnot the right shape",
            "created_at": "2026-04-23T00:00:00Z",
            "author": "ryan",
        }),
        serde_json::json!({
            "body": "**pickup_paraphrase** (dev pickup-gate):\n\nsecond pickup",
            "created_at": "2026-04-24T00:00:00Z",
            "author": "dev",
        }),
    ];
    let events = build_timeline(&story, &comments);
    let gates: Vec<&str> = events.iter().map(|e| e["gate"].as_str().unwrap()).collect();
    assert_eq!(
        gates,
        vec!["filed", "pickup", "delivered", "rejected", "pickup"]
    );
}

#[derive(Clone)]
struct TimelineTracker {
    story: serde_json::Value,
    comments: Vec<serde_json::Value>,
}

impl StoryTracker for TimelineTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        Ok(format!("#{number} closed"))
    }
    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} +{label}"))
    }
    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} -{label}"))
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
async fn handler_returns_json_array_of_events() {
    let tracker = TimelineTracker {
        story: serde_json::json!({
            "number": 418,
            "created_at": "2026-04-20T00:00:00Z",
            "user": {"login": "ryan"},
        }),
        comments: vec![serde_json::json!({
            "body": "**pickup_paraphrase** (dev pickup-gate):\n\nscope statement",
            "created_at": "2026-04-21T00:00:00Z",
            "author": "dev",
        })],
    };
    let args = serde_json::json!({"number": 418});
    let result = handle_view_story_timeline(args.as_object().unwrap(), &tracker, None, None)
        .await
        .expect("handler success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["gate"], "filed");
    assert_eq!(arr[1]["gate"], "pickup");
}

#[tokio::test]
async fn handler_errors_when_story_not_found() {
    #[derive(Clone)]
    struct MissingTracker;
    impl StoryTracker for MissingTracker {
        async fn close_story(&self, _n: u64) -> Result<String, String> {
            Ok(String::new())
        }
        async fn add_label(&self, _n: u64, _l: &str) -> Result<String, String> {
            Ok(String::new())
        }
        async fn remove_label(&self, _n: u64, _l: &str) -> Result<String, String> {
            Ok(String::new())
        }
        async fn create_story(&self, _t: &str, _b: &str) -> Result<String, String> {
            Ok(String::new())
        }
        async fn list_stories(&self) -> Result<String, String> {
            Ok(String::new())
        }
        async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
            Ok(Vec::new())
        }
        async fn get_story(&self, _n: u64) -> Result<Option<serde_json::Value>, String> {
            Ok(None)
        }
        async fn comment_on_story(&self, _n: u64, _b: &str) -> Result<String, String> {
            Ok(String::new())
        }
        async fn list_story_comments(&self, _n: u64) -> Result<Vec<serde_json::Value>, String> {
            Ok(Vec::new())
        }
        async fn get_story_title(&self, _n: u64) -> Option<String> {
            None
        }
        async fn get_story_labels(&self, _n: u64) -> Vec<String> {
            Vec::new()
        }
        async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
            Ok(None)
        }
    }
    let args = serde_json::json!({"number": 9999});
    let result =
        handle_view_story_timeline(args.as_object().unwrap(), &MissingTracker, None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn handler_reads_from_board_when_wired_and_lazy_fetches_gate_comments() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, GateComment, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let filed_at = Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap();
    let story = Story::new_minimal(
        418,
        "title".into(),
        StoryState::Delivered,
        filed_at,
        "pm".into(),
    );
    adapter.seed_stories(vec![story]);
    adapter.seed_gate_comments(
        418,
        vec![GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("ran end-to-end".into()),
        }],
    );
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let tracker = crate::bridge::tracker::FakeTracker::default();
    let args = serde_json::json!({"number": 418});
    let result = handle_view_story_timeline(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orchestrator),
    )
    .await
    .expect("board-path handler success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["gate"], "filed");
    assert_eq!(arr[0]["actor"], "pm");
    assert_eq!(arr[1]["gate"], "delivered");
    assert_eq!(arr[1]["actor"], "dev");
    assert_eq!(arr[1]["payload"], "ran end-to-end");

    assert!(
        store.story(418).unwrap().gate_comments.is_some(),
        "lazy fetch must populate cache"
    );
}
