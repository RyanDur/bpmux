#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::product_management::board::BoardStore;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{Story, StoryState};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

async fn build_orchestrator(stories: Vec<Story>) -> (Arc<BoardStore>, Arc<dyn RefreshDriver>) {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    (store, orch)
}

fn iteration_seed(number: u64, name: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {name}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".into(), "ready".into()];
    s
}

fn feature_seed(number: u64) -> Story {
    Story::new_minimal(
        number,
        "a feature".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

fn args(iteration_number: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "iteration_number".into(),
        serde_json::json!(iteration_number),
    );
    m
}

#[tokio::test]
async fn pm_designates_current_iteration_sets_is_current_1540() {
    let (store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    handle_designate_current_iteration(&args(1552), "pm:claude-code", Some(&orch))
        .await
        .expect("designate should succeed");
    assert!(
        crate::product_management::board::adapters::github::is_current_from_labels(
            &store.story(1552).unwrap().raw_labels
        ),
        "the designated iteration carries is_current"
    );
}

#[tokio::test]
async fn designate_clears_prior_current_holder_replace_semantic_1540() {
    let (store, orch) = build_orchestrator(vec![
        iteration_seed(1398, "Microscopium"),
        iteration_seed(1552, "Carina"),
    ])
    .await;
    handle_designate_current_iteration(&args(1398), "pm:claude-code", Some(&orch))
        .await
        .expect("first designate");
    handle_designate_current_iteration(&args(1552), "pm:claude-code", Some(&orch))
        .await
        .expect("second designate");
    assert!(
        !crate::product_management::board::adapters::github::is_current_from_labels(
            &store.story(1398).unwrap().raw_labels
        ),
        "prior current holder is cleared (board-singleton replace-semantic)"
    );
    assert!(
        crate::product_management::board::adapters::github::is_current_from_labels(
            &store.story(1552).unwrap().raw_labels
        ),
        "the newly-designated iteration is current"
    );
}

#[tokio::test]
async fn designate_rejects_non_iteration_story_1540() {
    let (_store, orch) = build_orchestrator(vec![feature_seed(1500)]).await;
    let err = handle_designate_current_iteration(&args(1500), "pm:claude-code", Some(&orch))
        .await
        .expect_err("a non-Iteration story must be rejected");
    assert!(
        err.contains("not an Iteration"),
        "error names the story-type mismatch: {err}"
    );
}

#[tokio::test]
async fn designate_rejected_for_non_pm_1540() {
    let (_store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    handle_designate_current_iteration(&args(1552), "dev:claude-code", Some(&orch))
        .await
        .expect_err("designate is PM-lane; non-pm is rejected");
}
