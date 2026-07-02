#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_velocity::*;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{BoardState, Stage, Story, StoryId, StoryKind};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn orchestrator_with_seed(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn make_story(id: u64, story_type: StoryKind) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        BoardState::InProgress(Stage::Dev),
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    s.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_story_type(story_type),
    );
    s.story_type = story_type;
    s
}

#[tokio::test]
async fn refreshed_path_classifies_feature_via_typed_story_type() {
    let orch = orchestrator_with_seed(vec![make_story(901, StoryKind::Feature)]).await;
    assert!(is_feature_for_velocity(901, Some(&orch)));
}

#[tokio::test]
async fn refreshed_path_rejects_bug_via_typed_story_type() {
    let orch = orchestrator_with_seed(vec![make_story(902, StoryKind::Bug)]).await;
    assert!(!is_feature_for_velocity(902, Some(&orch)));
}

#[tokio::test]
async fn refreshed_path_rejects_chore_via_typed_story_type() {
    let orch = orchestrator_with_seed(vec![make_story(903, StoryKind::Chore)]).await;
    assert!(!is_feature_for_velocity(903, Some(&orch)));
}

#[tokio::test]
async fn returns_false_when_refresh_absent() {
    assert!(!is_feature_for_velocity(904, None));
}

#[tokio::test]
async fn returns_false_when_story_not_in_cache() {
    let orch = orchestrator_with_seed(vec![]).await;
    assert!(!is_feature_for_velocity(905, Some(&orch)));
}
