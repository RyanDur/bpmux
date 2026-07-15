#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use product_management::story::StoryId;
use protocol::ClientMessage;

#[tokio::test]
async fn view_board_requires_refresh_after_5l_fallback_retirement() {
    let select = crate::store::Select::new(std::sync::Weak::new());
    let result = handle_view_board(&serde_json::Map::new(), None, &select).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn view_board_auto_reingests_when_store_is_empty() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(505),
        "post-restart-recovery".into(),
        Work::Accepted,
        ts,
        "dev".into(),
    )]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orch);
    let select = store.select();
    assert!(
        orch.all_stories().is_empty(),
        "store must start empty (simulates post-restart cache-loss); got: {:?}",
        orch.all_stories()
    );
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch), &select)
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orch);
    let select = store.select();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch), &select)
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orch);
    let select = store.select();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch), &select)
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::iteration::{Iteration, IterationId};
    use product_management::story::{Stage, Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let stories = vec![
        Story::new_minimal(
            StoryId::new(101),
            "icebox-story".into(),
            Work::NotStarted,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            StoryId::new(102),
            "ready-story".into(),
            Work::NotStarted,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            StoryId::new(103),
            "in-progress-story".into(),
            Work::InProgress,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            StoryId::new(104),
            "delivered-story".into(),
            Work::Acceptance,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            StoryId::new(108),
            "accepted-story".into(),
            Work::Accepted,
            ts,
            "owner".into(),
        ),
    ];
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let current = Iteration {
        id: IterationId::new(900),
        title: String::new(),
        body: None,
        filed_at: ts,
        filed_by: "pm".into(),
        outcome: None,
        iteration_order: None,
        stories: vec![StoryId::new(102)],
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    };
    let store = crate::store::SeededStore::new(vec![], vec![current], vec![]);
    store.bind_refresh(&orch);
    let select = store.select();
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch), &select)
        .await
        .unwrap();

    for header in [
        "ICEBOX (1)",
        "READY (1)",
        "IN PROGRESS (1)",
        "DESK CHECK (0)",
        "ACCEPTANCE (1)",
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
async fn view_board_buckets_concluded_stories_into_retired_or_accepted_column() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{Stage, Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let retired_story = Story::new_minimal(
        StoryId::new(201),
        "abandoned-mid-flight".into(),
        Work::Retired,
        ts,
        "pm".into(),
    );
    let accepted_story = Story::new_minimal(
        StoryId::new(202),
        "accepted-closed".into(),
        Work::Accepted,
        ts,
        "pm".into(),
    );
    adapter.seed_stories(vec![retired_story, accepted_story]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orch);
    let select = store.select();
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch), &select)
        .await
        .unwrap();

    assert!(
        out.contains("IN PROGRESS (0)"),
        "a concluded story must NOT appear in the IN PROGRESS bucket: {out}"
    );
    assert!(
        out.contains("RETIRED (1)"),
        "a Retired story projects to the RETIRED bucket: {out}"
    );
    assert!(
        out.contains("ACCEPTED (1)"),
        "only the Accepted story stays in the ACCEPTED bucket: {out}"
    );
    assert!(
        out.contains("#201 abandoned-mid-flight"),
        "the Retired story is visible under RETIRED: {out}"
    );
    assert!(
        out.contains("#202 accepted-closed"),
        "Accepted story stays in ACCEPTED: {out}"
    );
}

#[tokio::test]
async fn close_story_migrated_path_emits_story_closed_event_with_reason() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(301),
        "won't fix story".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(301));
    args.insert(
        "reason".into(),
        serde_json::json!("superseded by #999 — won't ship"),
    );
    args.insert("disposition".into(), serde_json::json!("retire"));
    let result = handle_close_story(&args, "pm:claude-code", "pm", None, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = orchestrator.story(301).expect("story exists");
    assert_eq!(
        story.state,
        Work::Retired,
        "#1570: disposition=retire transitions state to Retired"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "#1570: TagAdded 'retired' + StateTransitioned + StoryClosed (state-label reaping is the transition export's concern now): {pushed:?}"
    );
    let labels = adapter.labels_for(301);
    assert!(
        !labels.iter().any(|l| l == "delivered") && labels.iter().any(|l| l == "retired"),
        "the transition export strips the stale delivered label and the retired marker survives the close; got {labels:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::StoryClosed {
                reason: Some(r),
                ..
            } if e.id == StoryId::new(301) && r == "superseded by #999 — won't ship"
        )),
        "StoryClosed event must be present with the reason payload"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::TagAdded { label, .. } if e.id == StoryId::new(301) && label == "retired"
        )),
        "TagAdded 'retired' event must fire for retire-class reason"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::StateTransitioned { new_state, .. }
                if e.id == StoryId::new(301)
                    && *new_state == product_management::story::Work::Retired
        )),
        "StateTransitioned → Retired must fire for retire-class reason"
    );
}

#[tokio::test]
async fn close_story_accept_disposition_transitions_state_to_accepted() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(302),
        "accept via close".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(302));
    args.insert(
        "reason".into(),
        serde_json::json!("owner verified the outcome directly"),
    );
    args.insert("disposition".into(), serde_json::json!("accept"));
    let result = handle_close_story(&args, "pm:claude-code", "pm", None, Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "accept-disposition close should succeed: {result:?}"
    );

    let story = orchestrator.story(302).expect("story exists");
    assert_eq!(
        story.state,
        Work::Accepted,
        "disposition=accept concludes the story in the workflow — closing IS transitioning to a terminal state, never a bare GH-side close"
    );

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::StateTransitioned { new_state, .. }
                if e.id == StoryId::new(302) && *new_state == Work::Accepted
        )),
        "StateTransitioned → Accepted must fire for accept-class close: {pushed:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::StoryClosed { reason: Some(r), .. }
                if e.id == StoryId::new(302) && r == "owner verified the outcome directly"
        )),
        "StoryClosed carries the close reason to the GH push: {pushed:?}"
    );
}
