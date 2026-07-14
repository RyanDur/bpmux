#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::dev_in_flight::*;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::sync::{RefreshDriver, SyncOrchestrator};
use chrono::{TimeZone, Utc};
use product_management::story::Story;
use product_management::story::StoryId;
use product_management::story::Work;
use std::sync::Arc;

fn seed(state: Work, number: u64) -> Story {
    Story::new_minimal(
        StoryId::new(number),
        format!("title-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

async fn driver(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn returns_false_when_no_story_in_flight() {
    let refresh = driver(vec![
        seed(Work::NotStarted, 1),
        seed(Work::Acceptance, 2),
        seed(Work::Accepted, 3),
    ])
    .await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}

#[tokio::test]
async fn returns_true_when_a_story_is_in_progress() {
    let refresh = driver(vec![
        seed(Work::NotStarted, 1),
        seed(Work::InProgress, 2),
        seed(Work::Acceptance, 3),
    ])
    .await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":true"));
}

#[tokio::test]
async fn response_contains_no_story_identifiers() {
    let refresh = driver(vec![seed(Work::InProgress, 1)]).await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(!result.contains("\"id\""));
    assert!(!result.contains("title"));
}

#[tokio::test]
async fn returns_false_on_empty_store() {
    let refresh = driver(vec![]).await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}
