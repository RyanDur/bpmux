#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{FreeComment, Story, StoryState};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::peer::PeerId;
use crate::board::sync::{SourceAdapter, SyncError};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn seed_story(number: u64, title: &str) -> Story {
    Story::new_minimal(
        number,
        title.to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn ingest_metadata_returns_seeded_stories() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(1, "first"), seed_story(2, "second")]);
    let stories = adapter.ingest_metadata().await.unwrap();
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
    let event = BoardEvent::StateTransitioned {
        meta: meta(),
        number: 10,
        new_state: StoryState::Delivered,
    };
    adapter.push_event(&event).await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert_eq!(pushed[0], event);
}

#[tokio::test]
async fn push_event_errs_when_configured_to_fail() {
    let adapter = InMemorySourceAdapter::new();
    adapter.set_push_failures(true);
    let event = BoardEvent::StateTransitioned {
        meta: meta(),
        number: 10,
        new_state: StoryState::Delivered,
    };
    let result = adapter.push_event(&event).await;
    assert!(matches!(result, Err(SyncError::SourceUnreachable(_))));
    assert!(
        adapter.pushed_events().is_empty(),
        "failed push should not record the event"
    );
}

#[tokio::test]
async fn multiple_pushed_events_preserve_order() {
    let adapter = InMemorySourceAdapter::new();
    let e1 = BoardEvent::StateTransitioned {
        meta: meta(),
        number: 1,
        new_state: StoryState::InProgress,
    };
    let e2 = BoardEvent::StateTransitioned {
        meta: meta(),
        number: 2,
        new_state: StoryState::Delivered,
    };
    adapter.push_event(&e1).await.unwrap();
    adapter.push_event(&e2).await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].story_number(), 1);
    assert_eq!(pushed[1].story_number(), 2);
}
