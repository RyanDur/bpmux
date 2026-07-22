#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::tools_stories::handle_reprioritize_outcome;
use chrono::{TimeZone, Utc};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use product_management::roadmap::selectors::RoadmapOutcomeCard;
use product_management::story::{Horizon, Story, StoryId, StoryKind, Work};
use serde_json::{Map, Value, json};
use source_codec::mapper::decode_outcome_from_story;
use std::sync::Arc;
use store::{AppAction, Dispatch};
use sync::SyncOrchestrator;
use tokio::sync::mpsc::UnboundedReceiver;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    Dispatch::recording()
}

fn upserted(rx: &mut UnboundedReceiver<AppAction>) -> Vec<Outcome> {
    let mut found = Vec::new();
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Outcome(OutcomeAction::Upserted(outcomes)) = action {
            found = outcomes;
        }
    }
    found
}

fn seed(id: u64, title: &str, story_type: StoryKind, labels: &[&str]) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = story_type;
    let mut seed_labels: Vec<String> = labels.iter().map(ToString::to_string).collect();
    seed_labels.push(source_codec::gh_label_codec::GhLabelCodec.encode_story_type(story_type));
    (s, seed_labels)
}

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> (Story, Vec<String>) {
    let s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    (s, labels.iter().map(ToString::to_string).collect())
}

fn outcomes_from(seeds: &[(Story, Vec<String>)]) -> Vec<Outcome> {
    seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("outcome")))
        .map(|(story, labels)| decode_outcome_from_story(story, labels))
        .collect()
}

fn merged(base: Vec<Outcome>, updates: Vec<Outcome>) -> Vec<Outcome> {
    let mut out = base;
    for u in updates {
        match out.iter().position(|o| o.id == u.id) {
            Some(index) => out[index] = u,
            None => out.push(u),
        }
    }
    out
}

fn args_full(outcome: u64, to: Option<&str>, position: Option<u32>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_id".into(), json!(outcome));
    if let Some(t) = to {
        m.insert("to".into(), json!(t));
    }
    if let Some(p) = position {
        m.insert("position".into(), json!(p));
    }
    m
}

fn roadmap_cards(select: &store::Select) -> Vec<RoadmapOutcomeCard> {
    product_management::roadmap::selectors::roadmap_cards(
        &select.select(|state| product_management::outcome::all(&state.outcomes)),
        &select.select(|state| product_management::iteration::all(&state.iterations)),
        workflow::current_workflow(),
    )
}

fn band_order(select: &store::Select, horizon: Horizon) -> Vec<u64> {
    roadmap_cards(select)
        .into_iter()
        .filter(|card| card.horizon == horizon)
        .map(|card| card.id.value())
        .collect()
}

async fn build_refresh(
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
    let store = store_test_support::SeededStore::new(outcomes_from(&seeds), vec![], vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    let select = store.select();
    (store, orch, select)
}

async fn refresh_with_outcomes(outcomes: Vec<Outcome>) -> store::Select {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (orch, select) = store_test_support::build_seeded_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        outcomes,
        vec![],
        vec![],
    );
    orch.refresh_all().await.unwrap();
    select
}

fn find_outcome(outcomes: &[Outcome], id: u64) -> &Outcome {
    outcomes
        .iter()
        .find(|o| o.id == OutcomeId::new(id))
        .expect("outcome present in the optimistic dispatch")
}

fn args(outcome: u64, to: &str) -> Map<String, Value> {
    json!({ "outcome_id": outcome, "to": to })
        .as_object()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn pm_moves_an_outcome_to_a_new_band_and_the_horizon_rederives_1759() {
    let (_store, refresh, select) = build_refresh(vec![seed_outcome(
        700,
        "Agents reason from the substrate",
        &["outcome", "horizon.next"],
    )])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_reprioritize_outcome(
        &args(700, "now"),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("pm can reprioritize an outcome");
    assert!(
        result.contains("Now"),
        "the confirmation states the new band as the roadmap reads it: {result}"
    );
    assert_eq!(
        find_outcome(&upserted(&mut rx), 700).horizon(),
        Some(Horizon::Now),
        "the band-move re-derives the typed horizon from the swapped label (read<->write)"
    );
}

#[tokio::test]
async fn reprioritize_closes_the_read_write_loop_the_roadmap_view_reflects_the_new_band_1759() {
    let (_store, refresh, select) = build_refresh(vec![seed_outcome(
        700,
        "Agents reason from the substrate",
        &["outcome", "horizon.later"],
    )])
    .await;
    let before = roadmap_cards(&select);
    assert!(
        before
            .iter()
            .any(|c| c.horizon == Horizon::Later && c.id.value() == 700),
        "the outcome reads in the Later band before the move"
    );

    let (dispatch, mut rx) = dispatch_channel();
    handle_reprioritize_outcome(
        &args(700, "now"),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("pm reprioritizes the outcome");

    let after_select = refresh_with_outcomes(upserted(&mut rx)).await;
    let after = roadmap_cards(&after_select);
    assert!(
        after
            .iter()
            .any(|c| c.horizon == Horizon::Now && c.id.value() == 700),
        "the SAME derive /api/roadmap serves now places the outcome in Now — read<->write loop closed over the real substrate"
    );
    assert!(
        !after
            .iter()
            .any(|c| c.horizon == Horizon::Later && c.id.value() == 700),
        "the outcome no longer reads in Later — the band genuinely moved, not duplicated"
    );
}

#[tokio::test]
async fn reprioritize_outcome_is_pm_lane_non_pm_refused_1759() {
    let (_store, refresh, select) =
        build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.next"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_reprioritize_outcome(
        &args(700, "now"),
        "dev:claude-code",
        "dev",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("dev cannot reprioritize the roadmap (pm-lane)");
    assert!(err.contains("pm"), "refusal names the pm-lane: {err}");
    assert_eq!(
        select
            .select(|state| product_management::outcome::all(&state.outcomes))
            .iter()
            .find(|outcome| outcome.id == OutcomeId::new(700))
            .expect("outcome present")
            .horizon(),
        Some(Horizon::Next),
        "a refused write mutates nothing — the band is unchanged"
    );
}

#[tokio::test]
async fn reprioritize_aimed_at_a_non_outcome_teaches_the_boundary_1759() {
    let (_store, refresh, select) = build_refresh(vec![seed(
        700,
        "A plain story",
        StoryKind::Feature,
        &["enhancement"],
    )])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_reprioritize_outcome(
        &args(700, "now"),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("a roadmap-verb aimed at a non-outcome story is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of silently mutating a story: {err}"
    );
}

#[tokio::test]
async fn reprioritize_to_the_current_band_is_an_idempotent_noop_1759() {
    let (_store, refresh, select) = build_refresh(vec![seed_outcome(
        700,
        "Already here",
        &["outcome", "horizon.now"],
    )])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let result = handle_reprioritize_outcome(
        &args(700, "now"),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("re-firing to the current band is safe (idempotent)");
    assert!(
        result.contains("Now"),
        "the no-op still prints the current state, exit 0: {result}"
    );
    assert_eq!(
        select
            .select(|state| product_management::outcome::all(&state.outcomes))
            .iter()
            .find(|outcome| outcome.id == OutcomeId::new(700))
            .expect("outcome present")
            .horizon(),
        Some(Horizon::Now),
        "the band is unchanged on an idempotent re-fire"
    );
}

#[tokio::test]
async fn position_reorders_an_outcome_within_its_band_observed_through_the_derive_1759() {
    let seeds = vec![
        seed_outcome(700, "A", &["outcome", "horizon.now"]),
        seed_outcome(710, "B", &["outcome", "horizon.now"]),
        seed_outcome(720, "C", &["outcome", "horizon.now"]),
    ];
    let base = outcomes_from(&seeds);
    let (_store, refresh, select) = build_refresh(seeds).await;
    assert_eq!(
        band_order(&select, Horizon::Now),
        vec![700, 710, 720],
        "with no explicit order the band sorts by number"
    );

    let (dispatch, mut rx) = dispatch_channel();
    handle_reprioritize_outcome(
        &args_full(720, None, Some(1)),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("pm reorders C to the front of Now");

    let after_select = refresh_with_outcomes(merged(base, upserted(&mut rx))).await;
    assert_eq!(
        band_order(&after_select, Horizon::Now),
        vec![720, 700, 710],
        "C now reads first in Now — the within-band reorder reflects through the derive"
    );
}

#[tokio::test]
async fn position_overshoot_clamps_to_the_band_end_and_labels_it_1759() {
    let seeds = vec![
        seed_outcome(700, "A", &["outcome", "horizon.now"]),
        seed_outcome(710, "B", &["outcome", "horizon.now"]),
    ];
    let base = outcomes_from(&seeds);
    let (_store, refresh, select) = build_refresh(seeds).await;
    let (dispatch, mut rx) = dispatch_channel();
    let result = handle_reprioritize_outcome(
        &args_full(700, None, Some(9)),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("an overshoot clamps, never errors");
    assert!(
        result.contains("position 2 of 2"),
        "the overshoot clamps to the band end and labels the real slot count: {result}"
    );
    let after_select = refresh_with_outcomes(merged(base, upserted(&mut rx))).await;
    assert_eq!(
        band_order(&after_select, Horizon::Now),
        vec![710, 700],
        "A clamped to the last slot of Now"
    );
}

#[tokio::test]
async fn band_move_and_position_compose_in_one_verb_1759() {
    let seeds = vec![
        seed_outcome(700, "A", &["outcome", "horizon.now"]),
        seed_outcome(710, "B", &["outcome", "horizon.now"]),
        seed_outcome(720, "C", &["outcome", "horizon.next"]),
    ];
    let base = outcomes_from(&seeds);
    let (_store, refresh, select) = build_refresh(seeds).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_reprioritize_outcome(
        &args_full(720, Some("now"), Some(1)),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("C moves to Now AND lands at position 1");
    let after_select = refresh_with_outcomes(merged(base, upserted(&mut rx))).await;
    assert_eq!(
        band_order(&after_select, Horizon::Now),
        vec![720, 700, 710],
        "C moved bands and took the requested slot — --to and --position compose"
    );
    assert!(
        band_order(&after_select, Horizon::Next).is_empty(),
        "C no longer reads in Next"
    );
}

#[tokio::test]
async fn band_move_resets_a_stale_within_band_position_1759() {
    let (_store, refresh, select) = build_refresh(vec![seed_outcome(
        700,
        "A",
        &["outcome", "horizon.now", "horizon-order.5"],
    )])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_reprioritize_outcome(
        &args_full(700, Some("later"), None),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect("A moves to Later");
    assert_eq!(
        find_outcome(&upserted(&mut rx), 700).horizon_order(),
        None,
        "moving bands clears the stale within-band position so it doesn't mis-sort in the new band"
    );
}

#[tokio::test]
async fn reprioritize_with_neither_target_is_rejected_1759() {
    let (_store, refresh, select) =
        build_refresh(vec![seed_outcome(700, "A", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_reprioritize_outcome(
        &args_full(700, None, None),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
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
    let (_store, refresh, select) =
        build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_reprioritize_outcome(
        &args(700, "someday"),
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &dispatch,
        &select,
    )
    .await
    .expect_err("an invalid band is rejected");
    assert!(
        err.contains("now") && err.contains("next") && err.contains("later"),
        "the error names the valid bands: {err}"
    );
}
