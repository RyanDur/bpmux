#![allow(unused_imports)]

use super::super::tools_stories::*;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, Dispatch};
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use product_management::iteration::readiness::iteration_readiness;
use product_management::iteration::{Iteration, IterationAction, IterationId};
use product_management::story::{Story, StoryId, Work};
use source_codec::mapper::decode_iteration_from_story;
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

async fn build_orchestrator(
    seeds: Vec<(Story, Vec<String>)>,
) -> (
    crate::store::SeededStore,
    Arc<SyncOrchestrator>,
    crate::store::Select,
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
    let store = crate::store::SeededStore::new(vec![], iterations, vec![]);
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
async fn dev_ok_records_act_and_flips_dev_ok_readiness_1736() {
    let (_store, orch, select) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_dev_ok_iteration(
        &args(1552),
        "dev:claude-code",
        "dev",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
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
    let (_store, _orch, select) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let iteration = select
        .select(|state| {
            product_management::iteration::get(&state.iterations, IterationId::new(1552))
        })
        .unwrap();
    assert!(
        !iteration_readiness(&iteration, &bundled_default()).dev_ok,
        "no dev-OK on record → the check is unmet (no false-positive)"
    );
}

#[tokio::test]
async fn non_builder_role_is_refused_1736() {
    let (_store, orch, select) = build_orchestrator(vec![iteration_seed(1552, "Carina")]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_dev_ok_iteration(
        &args(1552),
        "pm:claude-code",
        "pm",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect_err("pm is not a configured iteration-builder");
    assert!(
        err.contains("record dev-OK for the iteration"),
        "the refusal names the role-gated operation; got: {err}"
    );
}

#[tokio::test]
async fn non_iteration_story_is_refused_1736() {
    let (_store, orch, select) = build_orchestrator(vec![feature_seed(1552)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_dev_ok_iteration(
        &args(1552),
        "dev:claude-code",
        "dev",
        None,
        Some(&orch),
        &dispatch,
        &select,
    )
    .await
    .expect_err("only an Iteration can be dev-OK'd");
    assert!(
        err.contains("not an Iteration"),
        "the refusal explains the story-type mismatch; got: {err}"
    );
}
