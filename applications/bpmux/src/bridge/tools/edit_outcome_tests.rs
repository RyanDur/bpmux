#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::bridge::tools_stories::handle_edit_outcome;
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

fn args(outcome: u64, title: Option<&str>, body: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_number".into(), json!(outcome));
    if let Some(t) = title {
        m.insert("title".into(), json!(t));
    }
    if let Some(b) = body {
        m.insert("body".into(), json!(b));
    }
    m
}

fn roadmap_name(refresh: &Arc<dyn RefreshDriver>, number: u64) -> Option<String> {
    let workflow = crate::workflow::current_workflow();
    crate::board::roadmap_view::derive_roadmap_view(&refresh.all_stories(), workflow)
        .into_iter()
        .flat_map(|b| b.outcomes)
        .find(|o| o.number == number)
        .map(|o| o.name)
}

#[tokio::test]
async fn editing_an_outcome_title_reflects_as_its_roadmap_name_1760() {
    let refresh = build_refresh(vec![seed(
        700,
        "Stale headline why",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    handle_edit_outcome(
        &args(700, Some("Agents reason from the shared substrate"), None),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm edits the outcome title");
    assert_eq!(
        roadmap_name(&refresh, 700).as_deref(),
        Some("Agents reason from the shared substrate"),
        "the title edit reflects as the roadmap name (read<->write through the same derive bpmux roadmap serves) — and the outcome stays in its Now band"
    );
}

#[tokio::test]
async fn editing_an_outcome_why_persists_for_the_bpmux_story_disclosure_read_1760() {
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    handle_edit_outcome(
        &args(700, None, Some("**Benefit:** agents plan from one substrate, no drift")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm edits the outcome why with a plain why-body — no ATP gate, since an outcome is not acceptance-test-bearing");
    assert_eq!(
        refresh.story(700).expect("present").body.as_deref(),
        Some("**Benefit:** agents plan from one substrate, no drift"),
        "the why persists as the disclosure-detail (reflected via bpmux story, not the token-frugal roadmap view)"
    );
}

#[tokio::test]
async fn edit_outcome_is_pm_lane_non_pm_refused_1760() {
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_edit_outcome(
        &args(700, Some("Y"), None),
        "dev:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("dev cannot edit an outcome (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn edit_outcome_aimed_at_a_non_outcome_teaches_the_boundary_1760() {
    let refresh = build_refresh(vec![seed(
        700,
        "A story",
        StoryType::Feature,
        &["enhancement"],
    )])
    .await;
    let err = handle_edit_outcome(
        &args(700, Some("Y"), None),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("outcome edit aimed at a non-outcome is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of editing a plain story: {err}"
    );
}

#[tokio::test]
async fn edit_outcome_with_no_change_is_rejected_1760() {
    let refresh = build_refresh(vec![seed(
        700,
        "X",
        StoryType::Outcome,
        &["outcome", "horizon.now"],
    )])
    .await;
    let err = handle_edit_outcome(&args(700, None, None), "pm:claude-code", Some(&refresh))
        .await
        .expect_err("editing with neither --title nor --body is a no-op request");
    assert!(
        err.contains("--title") && err.contains("--body"),
        "the error names the fix: {err}"
    );
}
