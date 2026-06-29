#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::product_management::board::BoardStore;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::SyncOrchestrator;
use crate::product_management::board::adapters::github::decode_iteration;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::readiness::iteration_readiness;
use crate::product_management::story::{Story, StoryState};
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

fn iteration_seed(number: u64, name: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {name}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".into(), "ready".into()];
    s.story_type = crate::product_management::story::StoryType::Iteration;
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
async fn designer_close_records_act_and_flips_review_readiness_1703() {
    let (store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    handle_close_pre_pickup_review(&args(1552), "designer:claude-code", Some(&orch))
        .await
        .expect("a configured iteration-reviewer can close the pre-pickup review");
    let iteration = store.iteration(1552).unwrap();
    assert_eq!(
        iteration.review_closers,
        vec!["designer".to_string()],
        "the close is recorded as a board-event re-derived into the closer-set"
    );
    assert!(
        iteration_readiness(&iteration, &[], &bundled_default()).review_closed,
        "closing the designer pre-pickup-review FLIPS the derived review-readiness-check"
    );
}

#[tokio::test]
async fn no_close_on_record_means_review_not_ready_1703() {
    let (store, _orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let iteration = store.iteration(1552).unwrap();
    assert!(
        !iteration_readiness(&iteration, &[], &bundled_default()).review_closed,
        "no review-close on record → the check is unmet (no false-positive)"
    );
}

#[tokio::test]
async fn non_reviewer_role_is_refused_1703() {
    let (_store, orch) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let err = handle_close_pre_pickup_review(&args(1552), "dev:claude-code", Some(&orch))
        .await
        .expect_err("dev is not a configured iteration-reviewer");
    assert!(
        err.contains("close the pre-pickup review"),
        "the refusal names the role-gated operation; got: {err}"
    );
}

#[tokio::test]
async fn non_iteration_story_is_refused_1703() {
    let (_store, orch) = build_orchestrator(vec![feature_seed(1552)]).await;
    let err = handle_close_pre_pickup_review(&args(1552), "designer:claude-code", Some(&orch))
        .await
        .expect_err("only an Iteration's pre-pickup review can be closed");
    assert!(
        err.contains("not an Iteration"),
        "the refusal explains the story-type mismatch; got: {err}"
    );
}
