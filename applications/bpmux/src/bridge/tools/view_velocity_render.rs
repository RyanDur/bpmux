#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_velocity::*;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{Story, StoryId, StoryKind, Work};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn orchestrator_with_seed(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.into_iter().map(|(s, _)| s).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn make_story(id: u64, story_type: StoryKind) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("story-{id}"),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    s.story_type = story_type;
    let labels = vec![
        crate::integrations::github::gh_label_codec::GhLabelCodec.encode_story_type(story_type),
    ];
    (s, labels)
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
