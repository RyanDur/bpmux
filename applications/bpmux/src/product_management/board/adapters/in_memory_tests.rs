#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::sync::{SourceAdapter, SyncError};
use crate::product_management::story::{FreeComment, Story, StoryState};

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

#[tokio::test]
async fn create_story_auto_assigns_first_number_when_empty() {
    let adapter = InMemorySourceAdapter::new();
    let number = adapter
        .create_story("first ever", "body", &[])
        .await
        .unwrap();
    assert_eq!(number, 1, "first story in empty adapter should be #1");
    let stories = adapter.ingest_metadata().await.unwrap();
    assert_eq!(stories.len(), 1);
    assert_eq!(stories[0].number, 1);
    assert_eq!(stories[0].title, "first ever");
    assert_eq!(adapter.fetch_story_body(1).await.unwrap(), "body");
}

#[tokio::test]
async fn create_story_auto_assigns_max_plus_one_when_seeded() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(7, "seed-a"), seed_story(11, "seed-b")]);
    let number = adapter.create_story("after-seed", "", &[]).await.unwrap();
    assert_eq!(
        number, 12,
        "next number must be (max-existing)+1, not gap-fill"
    );
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
    let event = BoardEvent::StoryCreated {
        meta: meta(),
        number: 99,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Icebox,
        story_type: crate::product_management::story::StoryType::Feature,
    };
    adapter.push_event(&event).await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
}

#[tokio::test]
async fn tag_added_surfaces_label_in_ingested_raw_labels() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "ordering-target")]);
    adapter
        .push_event(&BoardEvent::TagAdded {
            meta: meta(),
            number: 42,
            label: "iteration-Unscheduled.300".to_string(),
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    let story = stories.iter().find(|s| s.number == 42).unwrap();
    assert!(
        story
            .raw_labels
            .iter()
            .any(|l| l == "iteration-Unscheduled.300"),
        "TagAdded must surface the ordering label in raw_labels so reorder round-trips survive drain"
    );
}

#[tokio::test]
async fn tag_removed_drops_label_from_ingested_raw_labels() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "ordering-target")]);
    adapter.seed_labels(42, vec!["iteration-Unscheduled.300".to_string()]);
    adapter
        .push_event(&BoardEvent::TagRemoved {
            meta: meta(),
            number: 42,
            label: "iteration-Unscheduled.300".to_string(),
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    let story = stories.iter().find(|s| s.number == 42).unwrap();
    assert!(
        !story
            .raw_labels
            .iter()
            .any(|l| l == "iteration-Unscheduled.300"),
        "TagRemoved must drop the label from raw_labels"
    );
}

#[tokio::test]
async fn parent_epic_set_surfaces_in_ingested_metadata() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "child")]);
    adapter
        .push_event(&BoardEvent::ParentEpicSet {
            meta: meta(),
            number: 42,
            epic_number: 7,
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    let story = stories.iter().find(|s| s.number == 42).unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::parent_epic_from_labels(
            &story.raw_labels
        ),
        Some(7),
        "ParentEpicSet must surface parent_epic in the round-tripped Story"
    );
    let fetched = adapter.fetch_story_metadata(42).await.unwrap().unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::parent_epic_from_labels(
            &fetched.raw_labels
        ),
        Some(7),
        "ParentEpicSet must also surface via fetch_story_metadata"
    );
}

#[tokio::test]
async fn parent_epic_unset_clears_in_ingested_metadata() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "child")]);
    adapter
        .push_event(&BoardEvent::ParentEpicSet {
            meta: meta(),
            number: 42,
            epic_number: 7,
        })
        .await
        .unwrap();
    adapter
        .push_event(&BoardEvent::ParentEpicUnset {
            meta: meta(),
            number: 42,
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    let story = stories.iter().find(|s| s.number == 42).unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::parent_epic_from_labels(
            &story.raw_labels
        ),
        None,
        "ParentEpicUnset must clear parent_epic in the round-tripped Story"
    );
    let fetched = adapter.fetch_story_metadata(42).await.unwrap().unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::parent_epic_from_labels(
            &fetched.raw_labels
        ),
        None,
        "ParentEpicUnset must also clear via fetch_story_metadata"
    );
}

#[tokio::test]
async fn iteration_membership_set_surfaces_in_ingested_metadata() {
    let adapter = InMemorySourceAdapter::new();
    adapter.seed_stories(vec![seed_story(42, "member")]);
    adapter
        .push_event(&BoardEvent::IterationMembershipSet {
            meta: meta(),
            number: 42,
            iteration_number: 1300,
            position: 200,
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    let story = stories.iter().find(|s| s.number == 42).unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::iteration_membership_from_labels(
            &story.raw_labels
        ),
        Some((1300, 200)),
        "IterationMembershipSet must surface iteration_membership in the round-tripped Story"
    );
    let fetched = adapter.fetch_story_metadata(42).await.unwrap().unwrap();
    assert_eq!(
        crate::product_management::board::adapters::github::iteration_membership_from_labels(
            &fetched.raw_labels
        ),
        Some((1300, 200)),
        "IterationMembershipSet must also surface via fetch_story_metadata"
    );
}

#[tokio::test]
async fn membership_and_parent_epic_events_no_op_for_unseeded_story() {
    let adapter = InMemorySourceAdapter::new();
    adapter
        .push_event(&BoardEvent::IterationMembershipSet {
            meta: meta(),
            number: 999,
            iteration_number: 1,
            position: 1,
        })
        .await
        .unwrap();
    adapter
        .push_event(&BoardEvent::ParentEpicSet {
            meta: meta(),
            number: 999,
            epic_number: 1,
        })
        .await
        .unwrap();
    let stories = adapter.ingest_metadata().await.unwrap();
    assert!(
        stories.iter().all(|s| s.number != 999),
        "events for an unseeded story must no-op, never phantom-create"
    );
}
