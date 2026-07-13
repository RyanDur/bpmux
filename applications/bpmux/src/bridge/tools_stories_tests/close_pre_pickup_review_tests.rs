#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::integrations::github::mapper::decode_iteration_from_story;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, Dispatch};
use crate::sync::{RefreshDriver, SyncOrchestrator};
use chrono::{TimeZone, Utc};
use product_management::iteration::readiness::iteration_readiness;
use product_management::iteration::{Iteration, IterationAction, IterationId};
use product_management::story::{Story, StoryId, Work};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use workflow::bundled_default::bundled_default;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    Dispatch::recording()
}

fn iteration_upserted(rx: &mut UnboundedReceiver<AppAction>) -> Vec<Iteration> {
    let mut found = Vec::new();
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(iterations)) = action {
            found = iterations;
        }
    }
    found
}

async fn build_orchestrator(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let iterations: Vec<Iteration> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect();
    crate::store::bind_seeded_refresh(&orch, vec![], iterations, vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn iteration_seed(id: u64, name: &str) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, vec!["iteration".into(), "ready".into()])
}

fn feature_seed(id: u64) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        "a feature".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, Vec::new())
}

fn args(iteration_id: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("iteration_id".into(), serde_json::json!(iteration_id));
    m
}

#[tokio::test]
async fn designer_close_records_act_and_flips_review_readiness_1703() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_close_pre_pickup_review(
        &args(1552),
        "designer:claude-code",
        "designer",
        None,
        Some(&orch),
        &dispatch,
    )
    .await
    .expect("a configured iteration-reviewer can close the pre-pickup review");
    let iterations = iteration_upserted(&mut rx);
    let iteration = iterations
        .iter()
        .find(|it| it.id == IterationId::new(1552))
        .expect("the iteration is in the optimistic dispatch");
    assert_eq!(
        iteration.review_closers,
        vec!["designer".to_string()],
        "the close is recorded as a board-event re-derived into the closer-set"
    );
    assert!(
        iteration_readiness(iteration, &bundled_default()).review_closed,
        "closing the designer pre-pickup-review FLIPS the derived review-readiness-check"
    );
}

#[tokio::test]
async fn no_close_on_record_means_review_not_ready_1703() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let iteration = orch.iteration(IterationId::new(1552)).unwrap();
    assert!(
        !iteration_readiness(&iteration, &bundled_default()).review_closed,
        "no review-close on record → the check is unmet (no false-positive)"
    );
}

#[tokio::test]
async fn non_reviewer_role_is_refused_1703() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_close_pre_pickup_review(
        &args(1552),
        "dev:claude-code",
        "dev",
        None,
        Some(&orch),
        &dispatch,
    )
    .await
    .expect_err("dev is not a configured iteration-reviewer");
    assert!(
        err.contains("close the pre-pickup review"),
        "the refusal names the role-gated operation; got: {err}"
    );
}

#[tokio::test]
async fn non_iteration_story_is_refused_1703() {
    let orch = build_orchestrator(vec![feature_seed(1552)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_close_pre_pickup_review(
        &args(1552),
        "designer:claude-code",
        "designer",
        None,
        Some(&orch),
        &dispatch,
    )
    .await
    .expect_err("only an Iteration's pre-pickup review can be closed");
    assert!(
        err.contains("not an Iteration"),
        "the refusal explains the story-type mismatch; got: {err}"
    );
}
