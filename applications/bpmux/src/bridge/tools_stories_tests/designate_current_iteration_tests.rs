#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::decode_iteration_from_story;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::iteration::{Iteration, IterationAction, IterationId};
use crate::product_management::story::{Story, StoryId, Work};
use crate::store::{AppAction, Dispatch};
use crate::sync::{RefreshDriver, SyncOrchestrator};
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
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_designate_current_iteration(&args(1552), "pm:claude-code", Some(&orch), &dispatch)
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
    let orch = build_orchestrator(vec![
        current_iteration_seed(1398, "Microscopium"),
        iteration_seed(1552, "Carina"),
    ])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_designate_current_iteration(&args(1552), "pm:claude-code", Some(&orch), &dispatch)
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
    let orch = build_orchestrator(vec![feature_seed(1500)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err =
        handle_designate_current_iteration(&args(1500), "pm:claude-code", Some(&orch), &dispatch)
            .await
            .expect_err("a non-Iteration story must be rejected");
    assert!(
        err.contains("not an Iteration"),
        "error names the story-type mismatch: {err}"
    );
}

#[tokio::test]
async fn designate_rejected_for_non_pm_1540() {
    let orch = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, _rx) = dispatch_channel();
    handle_designate_current_iteration(&args(1552), "dev:claude-code", Some(&orch), &dispatch)
        .await
        .expect_err("designate is PM-lane; non-pm is rejected");
}
