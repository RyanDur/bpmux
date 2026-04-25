#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_story_timeline::handle_view_story_timeline;

#[tokio::test]
async fn handler_requires_board_and_refresh_after_5l_fallback_retirement() {
    let args = serde_json::json!({"number": 418});
    let result = handle_view_story_timeline(args.as_object().unwrap(), None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires board + refresh"));
}

#[tokio::test]
async fn handler_errors_when_story_not_in_board() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let args = serde_json::json!({"number": 9999});
    let result =
        handle_view_story_timeline(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
            .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in board"));
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

    let args = serde_json::json!({"number": 418});
    let result =
        handle_view_story_timeline(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
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

#[test]
fn build_timeline_from_board_sorts_chronologically() {
    use super::view_story_timeline::build_timeline_from_board;
    use crate::board::domain::{Gate, GateComment, Story, StoryState};
    use chrono::{TimeZone, Utc};

    let story = Story::new_minimal(
        500,
        "title".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 4, 25, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
    ];
    let events = build_timeline_from_board(&story, &gates);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[1]["gate"], "pickup");
    assert_eq!(events[2]["gate"], "delivered");
}

#[test]
fn build_timeline_from_board_renders_full_rework_cycle() {
    use super::view_story_timeline::build_timeline_from_board;
    use crate::board::domain::{Gate, GateComment, Story, StoryState};
    use chrono::{TimeZone, Utc};

    let story = Story::new_minimal(
        501,
        "title".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let gates = vec![
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 4, 21, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("first delivery".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap(),
            actor: "ryan".into(),
            payload: Some("not quite".into()),
        },
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
    ];
    let events = build_timeline_from_board(&story, &gates);
    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[1]["gate"], "pickup");
    assert_eq!(events[2]["gate"], "delivered");
    assert_eq!(events[3]["gate"], "rejected");
    assert_eq!(events[4]["gate"], "pickup");
    assert_eq!(events[2]["payload"], "first delivery");
    assert_eq!(events[3]["payload"], "not quite");
}
