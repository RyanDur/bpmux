#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::dev_in_flight::*;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{RefreshDriver, SyncOrchestrator};
use crate::product_management::story::BoardState;
use crate::product_management::story::Stage;
use crate::product_management::story::Story;
use crate::product_management::story::StoryId;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn seed(state: BoardState, number: u64) -> Story {
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
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn returns_false_when_no_story_in_flight() {
    let refresh = driver(vec![
        seed(BoardState::Ready, 1),
        seed(BoardState::Acceptance, 2),
        seed(BoardState::Accepted, 3),
    ])
    .await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}

#[tokio::test]
async fn returns_true_when_a_story_is_in_progress() {
    let refresh = driver(vec![
        seed(BoardState::Ready, 1),
        seed(BoardState::InProgress(Stage::Dev), 2),
        seed(BoardState::Acceptance, 3),
    ])
    .await;
    let result = handle_dev_in_flight(&refresh).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":true"));
}

#[tokio::test]
async fn response_contains_no_story_identifiers() {
    let refresh = driver(vec![seed(BoardState::InProgress(Stage::Dev), 1)]).await;
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
