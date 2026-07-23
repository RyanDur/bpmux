#![allow(
    unused_imports,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::super::tools_stories::*;
use product_management::story::StoryId;
use protocol::ClientMessage;

#[tokio::test]
async fn close_story_emits_story_closed_event_with_reason() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(301),
        "won't fix story".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(301));
    args.insert(
        "reason".into(),
        serde_json::json!("superseded by #999 — won't ship"),
    );
    args.insert("disposition".into(), serde_json::json!("retire"));
    let result = handle_close_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(301)))
        .expect("story exists");
    assert_eq!(
        story.state,
        Work::Retired,
        "#1570: disposition=retire transitions state to Retired"
    );

    assert!(
        adapter.pushed_events().is_empty(),
        "close_story returns optimistically — the GH push is deferred to the async GithubDrain reconciler: {:?}",
        adapter.pushed_events()
    );
    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(
        drained, 3,
        "the deferred TagAdded 'retired' + StateTransitioned + StoryClosed all reach the adapter on the reconciler's drain pass"
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
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(302),
        "accept via close".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(302));
    args.insert(
        "reason".into(),
        serde_json::json!("owner verified the outcome directly"),
    );
    args.insert("disposition".into(), serde_json::json!("accept"));
    let result = handle_close_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(
        result.is_ok(),
        "accept-disposition close should succeed: {result:?}"
    );

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(302)))
        .expect("story exists");
    assert_eq!(
        story.state,
        Work::Accepted,
        "disposition=accept concludes the story in the workflow — closing IS transitioning to a terminal state, never a bare GH-side close"
    );

    assert!(
        adapter.pushed_events().is_empty(),
        "close_story returns optimistically — nothing is pushed before the reconciler drains: {:?}",
        adapter.pushed_events()
    );
    orchestrator.drain_pass().await.unwrap();
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
