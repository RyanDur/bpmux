#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::dev_in_flight::*;
use crate::store::{AppAction, SeededStore};
use chrono::{TimeZone, Utc};
use product_management::story::Story;
use product_management::story::StoryId;
use product_management::story::Work;
use product_management::story::slice::StoryAction;

fn seed(state: Work, number: u64) -> Story {
    Story::new_minimal(
        StoryId::new(number),
        format!("title-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn store_with(stories: Vec<Story>) -> SeededStore {
    let store = SeededStore::new(vec![], vec![], vec![]);
    store.dispatch(AppAction::Story(StoryAction::Seeded(stories)));
    store
}

#[tokio::test]
async fn returns_false_when_no_story_in_flight() {
    let store = store_with(vec![
        seed(Work::NotStarted, 1),
        seed(Work::Acceptance, 2),
        seed(Work::Accepted, 3),
    ]);
    let result = handle_dev_in_flight(&store.select()).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}

#[tokio::test]
async fn returns_true_when_a_story_is_in_progress() {
    let store = store_with(vec![
        seed(Work::NotStarted, 1),
        seed(Work::InProgress, 2),
        seed(Work::Acceptance, 3),
    ]);
    let result = handle_dev_in_flight(&store.select()).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":true"));
}

#[tokio::test]
async fn response_contains_no_story_identifiers() {
    let store = store_with(vec![seed(Work::InProgress, 1)]);
    let result = handle_dev_in_flight(&store.select()).await.unwrap();
    assert!(!result.contains("\"id\""));
    assert!(!result.contains("title"));
}

#[tokio::test]
async fn returns_false_on_empty_store() {
    let store = store_with(vec![]);
    let result = handle_dev_in_flight(&store.select()).await.unwrap();
    assert!(result.contains("\"dev_in_flight\":false"));
}
