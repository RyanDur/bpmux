#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{
    handle_reorder_outcome_iteration, handle_unmap_outcome_iteration,
};
use crate::integrations::github::mapper::{decode_iteration_from_story, decode_outcome_from_story};
use crate::integrations::github::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, Dispatch};
use crate::sync::RefreshDriver;
use crate::sync::SourceAdapter;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationId};
use product_management::outcome::{Outcome, OutcomeId};
use product_management::story::{Story, StoryId, Work};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn iterations_from(seeds: &[(Story, Vec<String>)]) -> Vec<Iteration> {
    seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels, false))
        .collect()
}

fn dispatch() -> Dispatch {
    Dispatch::recording().0
}

fn dispatch_channel() -> (Dispatch, tokio::sync::mpsc::UnboundedReceiver<AppAction>) {
    Dispatch::recording()
}

fn iteration_upserted(rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppAction>) -> Vec<Iteration> {
    let mut found = Vec::new();
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(product_management::iteration::IterationAction::Upserted(
            iterations,
        )) = action
        {
            found.extend(iterations);
        }
    }
    found
}

fn outcome(id: u64, iterations: Vec<u64>) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Outcome #{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
    (s, vec!["outcome".into(), "horizon.now".into()])
}

fn iteration_for(id: u64, outcome_id: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration #{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    (s, vec!["iteration".to_string()])
}

async fn build_refresh(
    seeds: Vec<(Story, Vec<String>)>,
) -> (Arc<dyn RefreshDriver>, Arc<InMemorySourceAdapter>) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(crate::integrations::github::gh_label_codec::GhLabelCodec),
    ));
    let seeded: Vec<Outcome> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("outcome")))
        .map(|(story, labels)| decode_outcome_from_story(story, labels))
        .collect();
    crate::store::bind_seeded_refresh(&orch, seeded, iterations_from(&seeds), vec![]);
    orch.refresh_all().await.unwrap();
    (orch, adapter)
}

fn args(outcome: u64, iteration: u64) -> Map<String, Value> {
    json!({ "outcome_id": outcome, "iteration_id": iteration })
        .as_object()
        .cloned()
        .unwrap()
}

async fn reingested_outcome(adapter: &Arc<InMemorySourceAdapter>, id: u64) -> Outcome {
    adapter
        .ingest_metadata()
        .await
        .unwrap()
        .outcomes
        .into_iter()
        .find(|outcome| outcome.id == OutcomeId::new(id))
        .expect("the outcome re-decodes from the persisted substrate")
}

async fn outcome_iterations(adapter: &Arc<InMemorySourceAdapter>, id: u64) -> Vec<u64> {
    reingested_outcome(adapter, id)
        .await
        .iterations
        .iter()
        .map(|i| i.value())
        .collect()
}

async fn drill_iteration_ids(
    refresh: &Arc<dyn RefreshDriver>,
    adapter: &Arc<InMemorySourceAdapter>,
    id: u64,
) -> Vec<u64> {
    let workflow = workflow::current_workflow();
    let outcome = reingested_outcome(adapter, id).await;
    product_management::roadmap::drill::derive_outcome_drill_in(
        &outcome,
        &refresh.iterations(),
        workflow,
    )
    .iterations
    .iter()
    .map(|it| it.id.value())
    .collect()
}

#[tokio::test]
async fn unmap_removes_the_relation_and_the_drill_drops_the_iteration_1763() {
    let (refresh, adapter) = build_refresh(vec![
        outcome(700, vec![710, 720]),
        iteration_for(710, 700),
        iteration_for(720, 700),
    ])
    .await;
    assert_eq!(
        drill_iteration_ids(&refresh, &adapter, 700).await,
        vec![710, 720],
        "both iterations build the outcome before unmap"
    );

    handle_unmap_outcome_iteration(&args(700, 710), "pm", Some(&refresh), &dispatch())
        .await
        .expect("pm unmaps the iteration from the outcome");

    refresh.drain_pending().await.unwrap();

    assert_eq!(
        outcome_iterations(&adapter, 700).await,
        vec![720],
        "the parent's ordered iteration list drops the unmapped iteration"
    );
    assert_eq!(
        drill_iteration_ids(&refresh, &adapter, 700).await,
        vec![720],
        "the unmapped iteration drops from the outcome's drill (read<->write over the real derive)"
    );
}

#[tokio::test]
async fn unmap_leaves_the_iteration_itself_untouched_1763() {
    let (refresh, _adapter) =
        build_refresh(vec![outcome(700, vec![710]), iteration_for(710, 700)]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_unmap_outcome_iteration(&args(700, 710), "pm", Some(&refresh), &dispatch)
        .await
        .expect("pm unmaps");
    let dispatched = iteration_upserted(&mut rx);
    let iteration = dispatched
        .iter()
        .find(|it| it.id == IterationId::new(710))
        .expect("the iteration itself still exists — unmap removes only the relation");
    assert_eq!(
        iteration.builds_outcome(),
        None,
        "only the relation is removed — the iteration's outcome back-reference is cleared"
    );
    assert!(
        !iteration.complete,
        "the iteration ITSELF is untouched — not closed, not retired (the load-bearing falsifier)"
    );
}

#[tokio::test]
async fn reorder_repositions_the_iteration_within_the_outcome_1763() {
    let (refresh, adapter) = build_refresh(vec![
        outcome(700, vec![710, 720]),
        iteration_for(710, 700),
        iteration_for(720, 700),
    ])
    .await;
    assert_eq!(
        drill_iteration_ids(&refresh, &adapter, 700).await,
        vec![710, 720],
        "before: 710 leads 720"
    );
    let reorder = json!({ "outcome_id": 700, "iteration_id": 710, "position": 3 })
        .as_object()
        .cloned()
        .unwrap();
    handle_reorder_outcome_iteration(&reorder, "pm", Some(&refresh), &dispatch())
        .await
        .expect("pm reorders the iteration");

    refresh.drain_pending().await.unwrap();

    assert_eq!(
        outcome_iterations(&adapter, 700).await,
        vec![720, 710],
        "moving 710 to the end clamps to the band size and lands it last — the parent holds the new order in one write"
    );
    assert_eq!(
        drill_iteration_ids(&refresh, &adapter, 700).await,
        vec![720, 710],
        "the outcome's drill reflects the new priority-order — 710 moved behind 720"
    );
}

#[tokio::test]
async fn unmap_is_pm_lane_non_pm_refused_1763() {
    let (refresh, _adapter) =
        build_refresh(vec![outcome(700, vec![710]), iteration_for(710, 700)]).await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "dev", Some(&refresh), &dispatch())
        .await
        .expect_err("dev cannot unmap (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn unmap_rejects_an_unmapped_iteration_1763() {
    let (refresh, _adapter) =
        build_refresh(vec![outcome(700, vec![]), iteration_for(710, 999)]).await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "pm", Some(&refresh), &dispatch())
        .await
        .expect_err("an iteration not mapped to the outcome cannot be unmapped from it");
    assert!(
        err.contains("not mapped"),
        "the error explains it's not mapped: {err}"
    );
}

#[tokio::test]
async fn unmap_aimed_at_a_non_outcome_teaches_the_boundary_1763() {
    let feature = Story::new_minimal(
        StoryId::new(700),
        "[Feature] plain".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let (refresh, _adapter) = build_refresh(vec![
        (feature, vec!["feature".to_string()]),
        iteration_for(710, 700),
    ])
    .await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "pm", Some(&refresh), &dispatch())
        .await
        .expect_err("a non-outcome target is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary: {err}"
    );
}
