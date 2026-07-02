#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::product_management::board::adapters::github::decode_iteration_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{RefreshDriver, SyncOrchestrator};
use crate::product_management::iteration::readiness::iteration_readiness;
use crate::product_management::iteration::{Iteration, IterationAction, IterationId};
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::store::{AppAction, Dispatch};
use crate::workflow::bundled_default::bundled_default;
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    (Dispatch::new(tx), rx)
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

async fn build_orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    crate::store::bind_seeded_refresh(&orch, vec![], iterations, vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn iteration_seed(id: u64, name: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["iteration".into(), "ready".into()];
    s
}

fn feature_seed(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        "a feature".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    )
}

fn args(iteration_id: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("iteration_id".into(), serde_json::json!(iteration_id));
    m
}

#[tokio::test]
async fn dev_ok_records_act_and_flips_dev_ok_readiness_1736() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_dev_ok_iteration(&args(1552), "dev:claude-code", Some(&orch), &dispatch)
        .await
        .expect("a configured iteration-builder can record dev-OK");
    let iterations = iteration_upserted(&mut rx);
    let iteration = iterations
        .iter()
        .find(|it| it.id == IterationId::new(1552))
        .expect("the iteration is in the optimistic dispatch");
    assert_eq!(
        iteration.dev_ok_by,
        vec!["dev".to_string()],
        "the dev-OK is recorded as a board-event re-derived into the approver-set"
    );
    assert!(
        iteration_readiness(iteration, &bundled_default()).dev_ok,
        "dev OKing the iteration at IPM FLIPS the derived dev-OK readiness-check"
    );
}

#[tokio::test]
async fn no_dev_ok_on_record_means_not_ready_1736() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let iteration = orch.iteration(IterationId::new(1552)).unwrap();
    assert!(
        !iteration_readiness(&iteration, &bundled_default()).dev_ok,
        "no dev-OK on record → the check is unmet (no false-positive)"
    );
}

#[tokio::test]
async fn non_builder_role_is_refused_1736() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_dev_ok_iteration(&args(1552), "pm:claude-code", Some(&orch), &dispatch)
        .await
        .expect_err("pm is not a configured iteration-builder");
    assert!(
        err.contains("record dev-OK for the iteration"),
        "the refusal names the role-gated operation; got: {err}"
    );
}

#[tokio::test]
async fn non_iteration_story_is_refused_1736() {
    let orch = build_orchestrator(vec![feature_seed(1552)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_dev_ok_iteration(&args(1552), "dev:claude-code", Some(&orch), &dispatch)
        .await
        .expect_err("only an Iteration can be dev-OK'd");
    assert!(
        err.contains("not an Iteration"),
        "the refusal explains the story-type mismatch; got: {err}"
    );
}
