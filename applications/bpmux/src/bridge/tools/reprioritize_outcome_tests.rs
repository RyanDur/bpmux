#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_reprioritize_outcome;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Horizon, Story, StoryState, StoryType};
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
    s.raw_labels = labels.iter().map(ToString::to_string).collect();
    s.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_story_type(story_type),
    );
    s.story_type = story_type;
    s
}

fn args_full(outcome: u64, to: Option<&str>, position: Option<u32>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_number".into(), json!(outcome));
    if let Some(t) = to {
        m.insert("to".into(), json!(t));
    }
    if let Some(p) = position {
        m.insert("position".into(), json!(p));
    }
    m
}

fn band_order(refresh: &Arc<dyn RefreshDriver>, horizon: Horizon) -> Vec<u64> {
    let workflow = crate::workflow::current_workflow();
    crate::product_management::roadmap::view::derive_roadmap_view(&refresh.all_stories(), workflow)
        .into_iter()
        .find(|b| b.horizon == horizon)
        .map(|b| b.outcomes.iter().map(|o| o.number).collect())
        .unwrap_or_default()
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
        crate::product_management::board::adapters::github::horizon_from_labels(
            &refresh.story(700).expect("outcome present").raw_labels
        ),
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
    let before = crate::product_management::roadmap::view::derive_roadmap_view(
        &refresh.all_stories(),
        workflow,
    );
    assert!(
        before
            .iter()
            .any(|b| b.horizon == Horizon::Later && b.outcomes.iter().any(|o| o.number == 700)),
        "the outcome reads in the Later band before the move"
    );

    handle_reprioritize_outcome(&args(700, "now"), "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reprioritizes the outcome");

    let after = crate::product_management::roadmap::view::derive_roadmap_view(
        &refresh.all_stories(),
        workflow,
    );
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
        crate::product_management::board::adapters::github::horizon_from_labels(
            &refresh.story(700).expect("outcome present").raw_labels
        ),
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
        crate::product_management::board::adapters::github::horizon_from_labels(
            &refresh.story(700).expect("outcome present").raw_labels
        ),
        Some(Horizon::Now),
        "the band is unchanged on an idempotent re-fire"
    );
}

#[tokio::test]
async fn position_reorders_an_outcome_within_its_band_observed_through_the_derive_1759() {
    let refresh = build_refresh(vec![
        seed(700, "A", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(710, "B", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(720, "C", StoryType::Outcome, &["outcome", "horizon.now"]),
    ])
    .await;
    assert_eq!(
        band_order(&refresh, Horizon::Now),
        vec![700, 710, 720],
        "with no explicit order the band sorts by number"
    );

    handle_reprioritize_outcome(
        &args_full(720, None, Some(1)),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm reorders C to the front of Now");

    assert_eq!(
        band_order(&refresh, Horizon::Now),
        vec![720, 700, 710],
        "C now reads first in Now — the within-band reorder reflects through the derive"
    );
}

#[tokio::test]
async fn position_overshoot_clamps_to_the_band_end_and_labels_it_1759() {
    let refresh = build_refresh(vec![
        seed(700, "A", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(710, "B", StoryType::Outcome, &["outcome", "horizon.now"]),
    ])
    .await;
    let result = handle_reprioritize_outcome(
        &args_full(700, None, Some(9)),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("an overshoot clamps, never errors");
    assert!(
        result.contains("position 2 of 2"),
        "the overshoot clamps to the band end and labels the real slot count: {result}"
    );
    assert_eq!(
        band_order(&refresh, Horizon::Now),
        vec![710, 700],
        "A clamped to the last slot of Now"
    );
}

#[tokio::test]
async fn band_move_and_position_compose_in_one_verb_1759() {
    let refresh = build_refresh(vec![
        seed(700, "A", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(710, "B", StoryType::Outcome, &["outcome", "horizon.now"]),
        seed(720, "C", StoryType::Outcome, &["outcome", "horizon.next"]),
    ])
    .await;
    handle_reprioritize_outcome(
        &args_full(720, Some("now"), Some(1)),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("C moves to Now AND lands at position 1");
    assert_eq!(
        band_order(&refresh, Horizon::Now),
        vec![720, 700, 710],
        "C moved bands and took the requested slot — --to and --position compose"
    );
    assert!(
        band_order(&refresh, Horizon::Next).is_empty(),
        "C no longer reads in Next"
    );
}

#[tokio::test]
async fn band_move_resets_a_stale_within_band_position_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "A",
        StoryType::Outcome,
        &["outcome", "horizon.now", "horizon-order.5"],
    )])
    .await;
    handle_reprioritize_outcome(
        &args_full(700, Some("later"), None),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("A moves to Later");
    assert_eq!(
        crate::product_management::board::adapters::github::horizon_order_from_labels(
            &refresh.story(700).expect("present").raw_labels
        ),
        None,
        "moving bands clears the stale within-band position so it doesn't mis-sort in the new band"
    );
}

#[tokio::test]
async fn reprioritize_with_neither_target_is_rejected_1759() {
    let refresh = build_refresh(vec![seed(
        700,
        "A",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_reprioritize_outcome(
        &args_full(700, None, None),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("neither --to nor --position is a no-target");
    assert!(
        err.contains("--to") && err.contains("--position"),
        "the error names the fix (pass --to and/or --position): {err}"
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
