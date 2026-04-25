#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::get_story::handle_get_story;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Gate, GateComment, Story, StoryState};
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::bridge::tracker::FakeTracker;

async fn seeded_orchestrator(
    stories: Vec<Story>,
    gates: Vec<(u64, Vec<GateComment>)>,
) -> (Arc<BoardStore>, Arc<dyn RefreshDriver>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    for (n, gs) in gates {
        adapter.seed_gate_comments(n, gs);
    }
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();
    (store, orchestrator)
}

fn make_story(number: u64, state: StoryState, body: Option<&str>) -> Story {
    let mut s = Story::new_minimal(
        number,
        "title".into(),
        state,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    if let Some(b) = body {
        s.body = Some(b.to_string());
    }
    s
}

#[tokio::test]
async fn handler_requires_board_and_refresh_after_5l_fallback_retirement() {
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 443});
    let result = handle_get_story(args.as_object().unwrap(), &tracker, None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires board + refresh"));
}

#[tokio::test]
async fn handler_errors_when_story_not_in_board() {
    let (store, orch) = seeded_orchestrator(vec![], vec![]).await;
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 9999});
    let result = handle_get_story(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orch),
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in board"));
}

#[tokio::test]
async fn enriches_body_for_rejected_story_via_typed_gates() {
    let story = make_story(443, StoryState::Rejected, Some("original story body text"));
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("prior delivery ran end-to-end".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
            actor: "ryan".into(),
            payload: Some("channelKeyRef is not legitimate".into()),
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(443, gates)]).await;
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 443});
    let result = handle_get_story(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orch),
    )
    .await
    .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(body.starts_with("## Rework context"));
    assert!(body.contains("channelKeyRef is not legitimate"));
    assert!(body.contains("prior delivery ran end-to-end"));
    assert!(body.ends_with("original story body text"));
}

#[tokio::test]
async fn enriches_body_for_needs_fixes_story_via_typed_gates() {
    let story = make_story(200, StoryState::NeedsFixes, Some("original body"));
    let gates = vec![GateComment {
        gate: Gate::FailDeskCheck,
        ts: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
        actor: "designer".into(),
        payload: Some("failed spec".into()),
    }];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(200, gates)]).await;
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 200});
    let result = handle_get_story(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orch),
    )
    .await
    .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(body.contains("## Rework context"));
    assert!(body.contains("failed spec"));
}

#[tokio::test]
async fn does_not_enrich_body_for_non_rework_story() {
    let story = make_story(100, StoryState::Ready, Some("original body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 100});
    let result = handle_get_story(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orch),
    )
    .await
    .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "original body");
}

#[tokio::test]
async fn skips_enrichment_when_no_rework_gate_comments() {
    let story = make_story(50, StoryState::Rejected, Some("plain body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![(50, vec![])]).await;
    let tracker = FakeTracker::default();
    let args = serde_json::json!({"number": 50});
    let result = handle_get_story(
        args.as_object().unwrap(),
        &tracker,
        Some(&store),
        Some(&orch),
    )
    .await
    .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "plain body");
}
