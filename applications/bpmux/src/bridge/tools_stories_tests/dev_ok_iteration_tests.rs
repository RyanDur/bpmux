#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::product_management::board::BoardStore;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::domain::{Story, StoryState, StoryType};
use crate::product_management::iteration::readiness::iteration_readiness;
use crate::workflow::bundled_default::bundled_default;
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

fn iteration_seed(number: u64, theme: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {theme}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = StoryType::Iteration;
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
async fn dev_ok_records_act_and_flips_dev_ok_readiness_1736() {
    let (store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    handle_dev_ok_iteration(&args(1552), "dev:claude-code", Some(&orch))
        .await
        .expect("a configured iteration-builder can record dev-OK");
    let story = store.story(1552).unwrap();
    assert_eq!(
        story.iteration_dev_ok_by,
        vec!["dev".to_string()],
        "the dev-OK is recorded as a board-event re-derived into the approver-set"
    );
    assert!(
        iteration_readiness(&story, &[], &bundled_default()).dev_ok,
        "dev OKing the iteration at IPM FLIPS the derived dev-OK readiness-check"
    );
}

#[tokio::test]
async fn no_dev_ok_on_record_means_not_ready_1736() {
    let (store, _orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let story = store.story(1552).unwrap();
    assert!(
        !iteration_readiness(&story, &[], &bundled_default()).dev_ok,
        "no dev-OK on record → the check is unmet (no false-positive)"
    );
}

#[tokio::test]
async fn non_builder_role_is_refused_1736() {
    let (_store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let err = handle_dev_ok_iteration(&args(1552), "pm:claude-code", Some(&orch))
        .await
        .expect_err("pm is not a configured iteration-builder");
    assert!(
        err.contains("record dev-OK for the iteration"),
        "the refusal names the role-gated operation; got: {err}"
    );
}

#[tokio::test]
async fn non_iteration_story_is_refused_1736() {
    let (_store, orch) = build_orchestrator(vec![feature_seed(1552)]).await;
    let err = handle_dev_ok_iteration(&args(1552), "dev:claude-code", Some(&orch))
        .await
        .expect_err("only an Iteration can be dev-OK'd");
    assert!(
        err.contains("not an Iteration"),
        "the refusal explains the story-type mismatch; got: {err}"
    );
}
