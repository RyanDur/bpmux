#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::in_memory::InMemorySourceAdapter;
use product_management::epic::EpicId;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::{FreeComment, Story, StoryId, Work};
use source_sync::{SourceAdapter, SyncError};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap(),
    }
}

fn seed_story(id: u64, title: &str) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

async fn ingest_stories(adapter: &InMemorySourceAdapter) -> Vec<Story> {
    adapter.ingest_metadata().await.unwrap().items
}

#[tokio::test]
async fn the_source_adapter_port_is_object_safe_so_it_needs_no_erasure_wrapper() {
    let concrete = InMemorySourceAdapter::new();
    concrete.seed_stories(vec![seed_story(1, "reached through a trait object")]);
    let erased: std::sync::Arc<dyn SourceAdapter> = std::sync::Arc::new(concrete);

    let ingested = erased
        .ingest_metadata()
        .await
        .expect("a type-erased adapter ingests exactly as the concrete one does");

    assert!(
        ingested
            .items
            .iter()
            .any(|story| story.id == StoryId::new(1)),
        "a seeded story reaches the caller through the trait object's vtable"
    );
}

#[tokio::test]
async fn ingest_metadata_returns_seeded_stories() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(1, "first"), seed_story(2, "second")]);
    let stories = ingest_stories(&adapter).await;
    assert_eq!(stories.len(), 2);
    let titles: Vec<&str> = stories.iter().map(|s| s.title.as_str()).collect();
    assert!(titles.contains(&"first"));
    assert!(titles.contains(&"second"));
}

#[tokio::test]
async fn ingest_metadata_errs_when_configured_to_fail() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(1, "first")]);
    adapter.set_ingest_failures(true);
    let result = adapter.ingest_metadata().await;
    assert!(matches!(result, Err(SyncError::SourceUnreachable(_))));
}

#[tokio::test]
async fn fetch_story_body_returns_seeded_content() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_body(42, "body text".to_string());
    let body = adapter.fetch_story_body(42).await.unwrap();
    assert_eq!(body, "body text");
}

#[tokio::test]
async fn fetch_story_body_errs_when_missing() {
    let adapter = InMemorySourceAdapter::new();
    let result = adapter.fetch_story_body(999).await;
    assert!(matches!(result, Err(SyncError::Other(_))));
}

#[tokio::test]
async fn fetch_free_comments_returns_empty_when_none_seeded() {
    let adapter = InMemorySourceAdapter::new();
    let comments = adapter.fetch_free_comments(1).await.unwrap();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn fetch_free_comments_returns_seeded() {
    let adapter = InMemorySourceAdapter::new();
    let c = FreeComment {
        body: "hi".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
    };
    adapter.seed_free_comments(1, vec![c.clone()]);
    let fetched = adapter.fetch_free_comments(1).await.unwrap();
    assert_eq!(fetched, vec![c]);
}

#[tokio::test]
async fn push_event_records_the_event() {
    let adapter = InMemorySourceAdapter::new();
    let event = StoryEvent::new(
        meta(),
        StoryId::new(10),
        StoryEventKind::StateTransitioned {
            new_state: Work::Acceptance,
            placement: None,
        },
    );
    adapter
        .push_event(&event, &source_sync::PushContext::default())
        .await
        .unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0], event);
}

#[tokio::test]
async fn push_event_errs_when_configured_to_fail() {
    let adapter = InMemorySourceAdapter::new();
    adapter.set_push_failures(true);
    let event = StoryEvent::new(
        meta(),
        StoryId::new(10),
        StoryEventKind::StateTransitioned {
            new_state: Work::Acceptance,
            placement: None,
        },
    );
    let result = adapter
        .push_event(&event, &source_sync::PushContext::default())
        .await;
    assert!(matches!(result, Err(SyncError::SourceUnreachable(_))));
    assert!(
        adapter.pushed_events().is_empty(),
        "failed push should not record the event"
    );
}

#[tokio::test]
async fn multiple_pushed_events_preserve_order() {
    let adapter = InMemorySourceAdapter::new();
    let e1 = StoryEvent::new(
        meta(),
        StoryId::new(1),
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            placement: None,
        },
    );
    let e2 = StoryEvent::new(
        meta(),
        StoryId::new(2),
        StoryEventKind::StateTransitioned {
            new_state: Work::Acceptance,
            placement: None,
        },
    );
    adapter
        .push_event(&e1, &source_sync::PushContext::default())
        .await
        .unwrap();
    adapter
        .push_event(&e2, &source_sync::PushContext::default())
        .await
        .unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].story_id(), StoryId::new(1));
    assert_eq!(pushed[1].story_id(), StoryId::new(2));
}

#[tokio::test]
async fn create_story_auto_assigns_first_number_when_empty() {
    let adapter = InMemorySourceAdapter::new();
    let id = adapter
        .create_story("first ever", "body", &[])
        .await
        .unwrap();
    assert_eq!(id, 1, "first story in empty adapter should be #1");
    let stories = ingest_stories(&adapter).await;
    assert_eq!(stories.len(), 1);
    assert_eq!(stories[0].id, StoryId::new(1));
    assert_eq!(stories[0].title, "first ever");
    assert_eq!(adapter.fetch_story_body(1).await.unwrap(), "body");
}

#[tokio::test]
async fn create_story_auto_assigns_max_plus_one_when_seeded() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(7, "seed-a"), seed_story(11, "seed-b")]);
    let id = adapter.create_story("after-seed", "", &[]).await.unwrap();
    assert_eq!(id, 12, "next number must be (max-existing)+1, not gap-fill");
}

#[tokio::test]
async fn create_story_propagates_failure_when_push_failures_set() {
    let adapter = InMemorySourceAdapter::new();
    adapter.set_push_failures(true);
    let result = adapter.create_story("blocked", "", &[]).await;
    assert!(matches!(result, Err(SyncError::SourceUnreachable(_))));
}

#[tokio::test]
async fn push_event_accepts_story_created_variant() {
    let adapter = InMemorySourceAdapter::new();
    let event = StoryEvent::new(
        meta(),
        StoryId::new(99),
        StoryEventKind::StoryCreated {
            title: "t".to_string(),
            body: "b".to_string(),
            initial_state: Work::NotStarted,
            story_type: product_management::story::StoryKind::Feature,
        },
    );
    adapter
        .push_event(&event, &source_sync::PushContext::default())
        .await
        .unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
}

#[tokio::test]
async fn parent_epic_set_surfaces_in_ingested_metadata() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "child")]);
    adapter
        .push_event(
            &StoryEvent::new(
                meta(),
                StoryId::new(42),
                StoryEventKind::ParentEpicSet {
                    epic_id: EpicId::new(7),
                },
            ),
            &source_sync::PushContext::default(),
        )
        .await
        .unwrap();
    let stories = ingest_stories(&adapter).await;
    let story = stories.iter().find(|s| s.id == StoryId::new(42)).unwrap();
    assert_eq!(
        story.parent_epic,
        Some(EpicId::new(7)),
        "ParentEpicSet must surface parent_epic in the round-tripped Story"
    );
    let fetched = adapter.fetch_story_metadata(42).await.unwrap().unwrap();
    assert_eq!(
        fetched.0.parent_epic,
        Some(EpicId::new(7)),
        "ParentEpicSet must also surface via fetch_story_metadata"
    );
}

#[tokio::test]
async fn parent_epic_unset_clears_in_ingested_metadata() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "child")]);
    adapter
        .push_event(
            &StoryEvent::new(
                meta(),
                StoryId::new(42),
                StoryEventKind::ParentEpicSet {
                    epic_id: EpicId::new(7),
                },
            ),
            &source_sync::PushContext::default(),
        )
        .await
        .unwrap();
    adapter
        .push_event(
            &StoryEvent::new(meta(), StoryId::new(42), StoryEventKind::ParentEpicUnset),
            &source_sync::PushContext::default(),
        )
        .await
        .unwrap();
    let stories = ingest_stories(&adapter).await;
    let story = stories.iter().find(|s| s.id == StoryId::new(42)).unwrap();
    assert_eq!(
        story.parent_epic, None,
        "ParentEpicUnset must clear parent_epic in the round-tripped Story"
    );
    let fetched = adapter.fetch_story_metadata(42).await.unwrap().unwrap();
    assert_eq!(
        fetched.0.parent_epic, None,
        "ParentEpicUnset must also clear via fetch_story_metadata"
    );
}

#[tokio::test]
async fn parent_epic_event_no_ops_for_unseeded_story() {
    let adapter = InMemorySourceAdapter::new();
    adapter
        .push_event(
            &StoryEvent::new(
                meta(),
                StoryId::new(999),
                StoryEventKind::ParentEpicSet {
                    epic_id: EpicId::new(1),
                },
            ),
            &source_sync::PushContext::default(),
        )
        .await
        .unwrap();
    let stories = ingest_stories(&adapter).await;
    assert!(
        stories.iter().all(|s| s.id != StoryId::new(999)),
        "events for an unseeded story must no-op, never phantom-create"
    );
}
