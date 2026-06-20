#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Horizon, Story, StoryState, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::bridge::tools_stories::handle_reprioritize_outcome;
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn seed(number: u64, title: &str, story_type: StoryType, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        number,
        title.to_string(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = story_type;
    s.raw_labels = labels.iter().map(ToString::to_string).collect();
    s.horizon = crate::board::adapters::github::horizon_from_labels(&s.raw_labels);
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

fn args(outcome: u64, to: &str) -> Map<String, Value> {
    json!({ "outcome_number": outcome, "to": to })
        .as_object()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn pm_moves_an_outcome_to_a_new_band_and_the_horizon_rederives_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "Agents reason from the substrate",
        StoryType::Outcome,
        &["outcome", "horizon.next"],
    )])
    .await;
    let result = handle_reprioritize_outcome(&args(700, "now"), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm can reprioritize an outcome");
    assert!(
        result.contains("Now"),
        "the confirmation states the new band as the roadmap reads it: {result}"
    );
    assert_eq!(
        refresh.story(700).expect("outcome present").horizon,
        Some(Horizon::Now),
        "the band-move re-derives the typed horizon from the swapped label (read<->write)"
    );
}

#[tokio::test]
async fn reprioritize_closes_the_read_write_loop_the_roadmap_view_reflects_the_new_band_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "Agents reason from the substrate",
        StoryType::Outcome,
        &["outcome", "horizon.later"],
    )])
    .await;
    let workflow = crate::workflow::current_workflow();
    let before = crate::board::roadmap_view::derive_roadmap_view(&refresh.all_stories(), workflow);
    assert!(
        before
            .iter()
            .any(|b| b.horizon == Horizon::Later && b.outcomes.iter().any(|o| o.number == 700)),
        "the outcome reads in the Later band before the move"
    );

    handle_reprioritize_outcome(&args(700, "now"), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reprioritizes the outcome");

    let after = crate::board::roadmap_view::derive_roadmap_view(&refresh.all_stories(), workflow);
    assert!(
        after
            .iter()
            .any(|b| b.horizon == Horizon::Now && b.outcomes.iter().any(|o| o.number == 700)),
        "the SAME derive /api/roadmap serves now places the outcome in Now — read<->write loop closed over the real substrate"
    );
    assert!(
        !after
            .iter()
            .any(|b| b.horizon == Horizon::Later && b.outcomes.iter().any(|o| o.number == 700)),
        "the outcome no longer reads in Later — the band genuinely moved, not duplicated"
    );
}

#[tokio::test]
async fn reprioritize_outcome_is_pm_lane_non_pm_refused_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.next"],
    )])
    .await;
    let err = handle_reprioritize_outcome(&args(700, "now"), "dev:claude-code", Some(&refresh))
        .await
        .expect_err("dev cannot reprioritize the roadmap (pm-lane)");
    assert!(err.contains("pm"), "refusal names the pm-lane: {err}");
    assert_eq!(
        refresh.story(700).expect("outcome present").horizon,
        Some(Horizon::Next),
        "a refused write mutates nothing — the band is unchanged"
    );
}

#[tokio::test]
async fn reprioritize_aimed_at_a_non_outcome_teaches_the_boundary_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "A plain story",
        StoryType::Feature,
        &["enhancement"],
    )])
    .await;
    let err = handle_reprioritize_outcome(&args(700, "now"), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("a roadmap-verb aimed at a non-outcome story is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of silently mutating a story: {err}"
    );
}

#[tokio::test]
async fn reprioritize_to_the_current_band_is_an_idempotent_noop_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "Already here",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let result = handle_reprioritize_outcome(&args(700, "now"), "pm:claude-code", Some(&refresh))
        .await
        .expect("re-firing to the current band is safe (idempotent)");
    assert!(
        result.contains("Now"),
        "the no-op still prints the current state, exit 0: {result}"
    );
    assert_eq!(
        refresh.story(700).expect("outcome present").horizon,
        Some(Horizon::Now),
        "the band is unchanged on an idempotent re-fire"
    );
}

#[tokio::test]
async fn reprioritize_rejects_an_invalid_band_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_reprioritize_outcome(&args(700, "someday"), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("an invalid band is rejected");
    assert!(
        err.contains("now") && err.contains("next") && err.contains("later"),
        "the error names the valid bands: {err}"
    );
}
