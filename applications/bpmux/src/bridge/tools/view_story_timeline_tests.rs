#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_story_timeline::handle_view_story_timeline;

#[tokio::test]
async fn handler_requires_board_after_canonical_only_refactor() {
    let args = serde_json::json!({"number": 418});
    let result = handle_view_story_timeline(args.as_object().unwrap(), None, None).await;
    let err = result.expect_err("missing-board path errors");
    assert!(
        err.contains("requires board"),
        "post-#1165 slice 4: timeline requires canonical store; got: {err}"
    );
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
async fn handler_reads_gates_from_canonical_store() {
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
    let mut story = Story::new_minimal(
        418,
        "title".into(),
        StoryState::Delivered,
        filed_at,
        "pm".into(),
    );
    story.gate_comments = Some(vec![GateComment {
        gate: Gate::Delivered,
        ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
        actor: "dev".into(),
        payload: Some("ran end-to-end".into()),
    }]);
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"number": 418});
    let result =
        handle_view_story_timeline(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
            .await
            .expect("canonical-path handler success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["gate_comments_hydrated"], true);
    let events = parsed["events"].as_array().expect("events array");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[0]["actor"], "pm");
    assert_eq!(events[1]["gate"], "delivered");
    assert_eq!(events[1]["actor"], "dev");
    assert_eq!(events[1]["payload"], "ran end-to-end");
}

#[tokio::test]
async fn handler_returns_filed_only_with_unhydrated_flag_when_gate_comments_missing() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let story = Story::new_minimal(
        419,
        "title".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"number": 419});
    let result =
        handle_view_story_timeline(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
            .await
            .expect("canonical-path handler succeeds without live GH");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["gate_comments_hydrated"], false);
    let events = parsed["events"].as_array().expect("events array");
    assert_eq!(events.len(), 1, "filed event only when gates not hydrated");
    assert_eq!(events[0]["gate"], "filed");
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
