#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{
    handle_reorder_outcome_iteration, handle_unmap_outcome_iteration,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryState};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn outcome(number: u64) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Outcome #{number}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec!["outcome".into(), "horizon.now".into()];
    s
}

fn iteration_mapped(number: u64, outcome_number: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration #{number}"),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels.push("iteration".to_string());
    s.raw_labels
        .push(format!("parent-outcome-#{outcome_number}.{position}"));
    s.body = Some("## composed".into());
    s
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
}

fn args(outcome: u64, iteration: u64) -> Map<String, Value> {
    json!({ "outcome_number": outcome, "iteration_number": iteration })
        .as_object()
        .cloned()
        .unwrap()
}

fn drill_iteration_numbers(refresh: &Arc<dyn RefreshDriver>, outcome: u64) -> Vec<u64> {
    let workflow = crate::workflow::current_workflow();
    crate::product_management::roadmap::drill::derive_outcome_drill_in(
        outcome,
        &refresh.all_stories(),
        workflow,
    )
    .iterations
    .iter()
    .map(|it| it.number)
    .collect()
}

#[tokio::test]
async fn unmap_removes_the_relation_and_the_drill_drops_the_iteration_1763() {
    let refresh = build_refresh(vec![
        outcome(700),
        iteration_mapped(710, 700, 1),
        iteration_mapped(720, 700, 2),
    ])
    .await;
    assert_eq!(
        drill_iteration_numbers(&refresh, 700),
        vec![710, 720],
        "both iterations build the outcome before unmap"
    );

    handle_unmap_outcome_iteration(&args(700, 710), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm unmaps the iteration from the outcome");

    assert_eq!(
        drill_iteration_numbers(&refresh, 700),
        vec![720],
        "the unmapped iteration drops from the outcome's drill (read<->write over the real derive)"
    );
}

#[tokio::test]
async fn unmap_leaves_the_iteration_itself_untouched_1763() {
    let refresh = build_refresh(vec![outcome(700), iteration_mapped(710, 700, 1)]).await;
    handle_unmap_outcome_iteration(&args(700, 710), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm unmaps");
    let iteration = refresh.story(710).expect("present");
    assert_eq!(
        crate::product_management::board::adapters::github::outcome_membership_from_labels(
            &iteration.raw_labels
        ),
        None,
        "only the relation is removed"
    );
    assert_eq!(
        iteration.state,
        StoryState::Ready,
        "the iteration ITSELF is untouched — not closed, not retired (the load-bearing falsifier)"
    );
    assert!(!iteration.is_closed, "unmap does not close the iteration");
}

#[tokio::test]
async fn reorder_repositions_the_iteration_within_the_outcome_1763() {
    let refresh = build_refresh(vec![
        outcome(700),
        iteration_mapped(710, 700, 1),
        iteration_mapped(720, 700, 2),
    ])
    .await;
    assert_eq!(
        drill_iteration_numbers(&refresh, 700),
        vec![710, 720],
        "before: 710 (pos 1) leads 720 (pos 2)"
    );
    let reorder = json!({ "outcome_number": 700, "iteration_number": 710, "position": 3 })
        .as_object()
        .cloned()
        .unwrap();
    handle_reorder_outcome_iteration(&reorder, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reorders the iteration");
    assert_eq!(
        crate::product_management::board::adapters::github::outcome_membership_from_labels(
            &refresh.story(710).expect("present").raw_labels
        ),
        Some((700, 3)),
        "the iteration reads its new priority-order within the outcome"
    );
    assert_eq!(
        drill_iteration_numbers(&refresh, 700),
        vec![720, 710],
        "the outcome's drill reflects the new priority-order — 710 moved behind 720 (distinct positions, deterministic)"
    );
}

#[tokio::test]
async fn unmap_is_pm_lane_non_pm_refused_1763() {
    let refresh = build_refresh(vec![outcome(700), iteration_mapped(710, 700, 1)]).await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "dev:claude-code", Some(&refresh))
        .await
        .expect_err("dev cannot unmap (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn unmap_rejects_an_unmapped_iteration_1763() {
    let refresh = build_refresh(vec![outcome(700), iteration_mapped(710, 999, 1)]).await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("an iteration not mapped to the outcome cannot be unmapped from it");
    assert!(
        err.contains("not mapped"),
        "the error explains it's not mapped: {err}"
    );
}

#[tokio::test]
async fn unmap_aimed_at_a_non_outcome_teaches_the_boundary_1763() {
    let mut feature = Story::new_minimal(
        700,
        "[Feature] plain".into(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    feature.raw_labels.push("feature".to_string());
    let refresh = build_refresh(vec![feature, iteration_mapped(710, 700, 1)]).await;
    let err = handle_unmap_outcome_iteration(&args(700, 710), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("a non-outcome target is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary: {err}"
    );
}
