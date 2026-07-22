#![allow(unused_imports)]

use super::super::tools_stories::*;
use chrono::{TimeZone, Utc};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::iteration::{Iteration, IterationAction, IterationId};
use product_management::story::{Story, StoryId, Work};
use source_codec::gh_label_codec::GhLabelCodec;
use source_codec::mapper::decode_iteration_from_story;
use std::sync::Arc;
use store::{AppAction, Dispatch};
use sync::SyncOrchestrator;
use tokio::sync::mpsc::UnboundedReceiver;

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

async fn build_orchestrator(
    seeds: Vec<(Story, Vec<String>)>,
) -> (
    store_test_support::SeededStore,
    Arc<SyncOrchestrator>,
    store::Select,
) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let iterations: Vec<Iteration> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect();
    let store = store_test_support::SeededStore::new(vec![], iterations, vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    let select = store.select();
    (store, orch, select)
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

fn current_iteration_seed(id: u64, name: &str) -> (Story, Vec<String>) {
    let (s, mut labels) = iteration_seed(id, name);
    labels.push(GhLabelCodec.encode_current_iteration());
    (s, labels)
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
async fn pm_designates_current_iteration_sets_is_current_1540() {
    let (_store, orch, select) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_designate_current_iteration(
        &args(1552),
        "pm:claude-code",
        "pm",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect("designate should succeed");
    let iterations = iteration_upserted(&mut rx);
    let designated = iterations
        .iter()
        .find(|it| it.id == IterationId::new(1552))
        .expect("the designated iteration is in the optimistic dispatch");
    assert!(
        designated.is_current,
        "the designated iteration carries is_current"
    );
}

#[tokio::test]
async fn designate_clears_prior_current_holder_replace_semantic_1540() {
    let (_store, orch, select) = build_orchestrator(vec![
        current_iteration_seed(1398, "Microscopium"),
        iteration_seed(1552, "Carina"),
    ])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_designate_current_iteration(
        &args(1552),
        "pm:claude-code",
        "pm",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect("designate");
    let iterations = iteration_upserted(&mut rx);
    let prior = iterations
        .iter()
        .find(|it| it.id == IterationId::new(1398))
        .expect("the prior current holder is in the optimistic dispatch");
    assert!(
        !prior.is_current,
        "prior current holder is cleared (board-singleton replace-semantic)"
    );
    let designated = iterations
        .iter()
        .find(|it| it.id == IterationId::new(1552))
        .expect("the newly-designated iteration is in the optimistic dispatch");
    assert!(
        designated.is_current,
        "the newly-designated iteration is current"
    );
}

#[tokio::test]
async fn designate_rejects_non_iteration_story_1540() {
    let (_store, orch, select) = build_orchestrator(vec![feature_seed(1500)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_designate_current_iteration(
        &args(1500),
        "pm:claude-code",
        "pm",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect_err("a non-Iteration story must be rejected");
    assert!(
        err.contains("not an Iteration"),
        "error names the story-type mismatch: {err}"
    );
}

#[tokio::test]
async fn designate_rejected_for_non_pm_1540() {
    let (_store, orch, select) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, _rx) = dispatch_channel();
    handle_designate_current_iteration(
        &args(1552),
        "dev:claude-code",
        "dev",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect_err("designate is PM-lane; non-pm is rejected");
}
