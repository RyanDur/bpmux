#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::dev_in_flight::*;
use crate::product_management::board::BoardStore;
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

#[tokio::test]
async fn returns_false_when_no_story_in_flight() {
    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![
        seed(BoardState::Ready, 1),
        seed(BoardState::Delivered, 2),
        seed(BoardState::Accepted, 3),
    ]);
    let result = handle_dev_in_flight(&store).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}

#[tokio::test]
async fn returns_true_when_a_story_is_in_progress() {
    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![
        seed(BoardState::Ready, 1),
        seed(BoardState::InProgress(Stage::Dev), 2),
        seed(BoardState::Delivered, 3),
    ]);
    let result = handle_dev_in_flight(&store).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":true"));
}

#[tokio::test]
async fn response_contains_no_story_identifiers() {
    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![seed(BoardState::InProgress(Stage::Dev), 1)]);
    let result = handle_dev_in_flight(&store).await.unwrap();
    assert!(!result.contains("\"id\""));
    assert!(!result.contains("title"));
}

#[tokio::test]
async fn returns_false_on_empty_store() {
    let store = Arc::new(BoardStore::new());
    let result = handle_dev_in_flight(&store).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}
