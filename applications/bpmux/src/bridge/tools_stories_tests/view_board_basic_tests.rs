#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::{
    connected_ws_sink_for_test, connected_ws_sink_with_capture, test_ci_state_store_green,
};
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn view_board_requires_refresh_after_5l_fallback_retirement() {
    let result = handle_view_board(&serde_json::Map::new(), None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn view_board_auto_reingests_when_store_is_empty() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    adapter.seed_stories(vec![Story::new_minimal(
        505,
        "post-restart-recovery".into(),
        StoryState::Accepted,
        ts,
        "dev".into(),
    )]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    assert!(
        orch.all_stories().is_empty(),
        "store must start empty (simulates post-restart cache-loss); got: {:?}",
        orch.all_stories()
    );
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#505 post-restart-recovery"),
        "view_board should have triggered re-ingest from adapter; output:\n{out}"
    );
    assert!(
        !orch.all_stories().is_empty(),
        "post-call store should be populated; auto-recovery completed"
    );
}

#[tokio::test]
async fn view_board_handles_refresh_succeeding_with_empty_result_without_errors() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .expect(
            "view_board must return Ok even when adapter source-of-truth has 0 stories — \
             empty-result must be honest, not error",
        );

    assert!(
        out.contains("ICEBOX (0)"),
        "view_board returns honest-empty when source has no stories; output:\n{out}"
    );
    assert!(
        orch.all_stories().is_empty(),
        "post-call store remains empty when source-of-truth is empty (silently-empty case logged via diag per #506 observability fix)"
    );
}

#[tokio::test]
async fn view_board_handles_refresh_failing_returns_honest_empty_with_diag_log() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .expect("view_board must return Ok even when adapter ingest fails — fail-soft");

    assert!(
        out.contains("ICEBOX (0)"),
        "view_board returns honest-empty when ingest fails; output:\n{out}"
    );
    assert!(
        orch.all_stories().is_empty(),
        "store stays empty when ingest fails; diag log captures the failure for #506 observability"
    );
}

#[tokio::test]
async fn view_board_buckets_stories_by_story_state() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let stories = vec![
        Story::new_minimal(
            101,
            "icebox-story".into(),
            StoryState::Icebox,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            102,
            "ready-story".into(),
            StoryState::Ready,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            103,
            "in-progress-story".into(),
            StoryState::InProgress,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            104,
            "delivered-story".into(),
            StoryState::Delivered,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            108,
            "accepted-story".into(),
            StoryState::Accepted,
            ts,
            "ryan".into(),
        ),
    ];
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();

    for header in [
        "ICEBOX (1)",
        "READY (1)",
        "IN PROGRESS (1)",
        "DESK CHECK (0)",
        "DELIVERED (1)",
        "ACCEPTED (1)",
    ] {
        assert!(
            out.contains(header),
            "expected section header {header:?} in output:\n{out}"
        );
    }
    assert!(out.contains("#101 icebox-story"));
    assert!(out.contains("#102 ready-story"));
    assert!(out.contains("#108 accepted-story"));
}

#[tokio::test]
async fn view_board_buckets_closed_stories_into_retired_or_accepted_column() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut closed_in_progress = Story::new_minimal(
        201,
        "abandoned-mid-flight".into(),
        StoryState::InProgress,
        ts,
        "pm".into(),
    );
    closed_in_progress.is_closed = true;
    let mut accepted_closed = Story::new_minimal(
        202,
        "accepted-closed".into(),
        StoryState::Accepted,
        ts,
        "pm".into(),
    );
    accepted_closed.is_closed = true;
    adapter.seed_stories(vec![closed_in_progress, accepted_closed]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();

    assert!(
        out.contains("IN PROGRESS (0)"),
        "is_closed InProgress story must NOT appear in IN PROGRESS bucket: {out}"
    );
    assert!(
        out.contains("RETIRED (1)"),
        "closed-non-Accepted story projects to RETIRED bucket: {out}"
    );
    assert!(
        out.contains("ACCEPTED (1)"),
        "only Accepted story stays in ACCEPTED bucket: {out}"
    );
    assert!(
        out.contains("#201 abandoned-mid-flight"),
        "closed-non-Accepted story visible under RETIRED: {out}"
    );
    assert!(
        out.contains("#202 accepted-closed"),
        "Accepted story stays in ACCEPTED: {out}"
    );
}

#[tokio::test]
async fn close_story_migrated_path_emits_story_closed_event_with_reason() {
    use crate::board::BoardEvent;
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
        301,
        "won't fix story".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(301));
    args.insert(
        "reason".into(),
        serde_json::json!("superseded by #999 — won't ship"),
    );
    let result = handle_close_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(301).expect("story exists");
    assert!(story.is_closed, "is_closed flag must be set");
    assert_eq!(
        story.state,
        StoryState::Retired,
        "post-#1377: retire-class reason transitions state to Retired"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        4,
        "post-#1377: 1 TagRemoved 'delivered' + 1 TagAdded 'retired' + 1 StateTransitioned + 1 StoryClosed (reason='superseded …' classifies retire-class): {pushed:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::StoryClosed {
                number: 301,
                reason: Some(r),
                ..
            } if r == "superseded by #999 — won't ship"
        )),
        "StoryClosed event must be present with the reason payload"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::TagAdded { number: 301, label, .. } if label == "retired"
        )),
        "TagAdded 'retired' event must fire for retire-class reason"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::StateTransitioned { number: 301, new_state, .. }
                if *new_state == crate::board::domain::StoryState::Retired
        )),
        "StateTransitioned → Retired must fire for retire-class reason"
    );
}
