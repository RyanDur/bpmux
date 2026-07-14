#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_edit_outcome;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, Dispatch};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use chrono::{TimeZone, Utc};
use product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use product_management::story::{Horizon, Story, StoryId, StoryKind, Work};
use serde_json::{Map, Value, json};
use source_codec::mapper::decode_outcome_from_story;
use std::sync::Arc;
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

async fn build_refresh(seeds: Vec<(Story, Vec<String>)>) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    for (story, labels) in &seeds {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeds.iter().map(|(s, _)| s.clone()).collect());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let seeded: Vec<Outcome> = seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("outcome")))
        .map(|(story, labels)| decode_outcome_from_story(story, labels))
        .collect();
    crate::store::bind_seeded_refresh(&orch, seeded, vec![], vec![]);
    orch.refresh_all().await.unwrap();
    orch
}

fn args(outcome: u64, title: Option<&str>, body: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("outcome_id".into(), json!(outcome));
    if let Some(t) = title {
        m.insert("title".into(), json!(t));
    }
    if let Some(b) = body {
        m.insert("body".into(), json!(b));
    }
    m
}

#[tokio::test]
async fn editing_an_outcome_title_reflects_as_its_roadmap_name_1760() {
    let refresh = build_refresh(vec![seed_outcome(
        700,
        "Stale headline why",
        &["outcome", "horizon.now"],
    )])
    .await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_edit_outcome(
        &args(700, Some("Agents reason from the shared substrate"), None),
        "pm",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm edits the outcome title");
    let outcomes = upserted(&mut rx);
    let outcome = outcomes
        .iter()
        .find(|o| o.id == OutcomeId::new(700))
        .expect("the edit dispatches the updated outcome optimistically");
    assert_eq!(
        outcome.title, "Agents reason from the shared substrate",
        "the title edit is the roadmap name (read<->write through the same outcome the bpmux roadmap serves)"
    );
    assert_eq!(
        outcome.horizon(),
        Some(Horizon::Now),
        "and the outcome stays in its Now band"
    );
}

#[tokio::test]
async fn editing_an_outcome_why_persists_for_the_bpmux_story_disclosure_read_1760() {
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_edit_outcome(
        &args(700, None, Some("**Benefit:** agents plan from one substrate, no drift")),
        "pm",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("pm edits the outcome why with a plain why-body — no ATP gate, since an outcome is not acceptance-test-bearing");
    let outcomes = upserted(&mut rx);
    let outcome = outcomes
        .iter()
        .find(|o| o.id == OutcomeId::new(700))
        .expect("present");
    assert_eq!(
        outcome.body.as_deref(),
        Some("**Benefit:** agents plan from one substrate, no drift"),
        "the why persists as the disclosure-detail (reflected via the optimistic outcome dispatch)"
    );
}

#[tokio::test]
async fn edit_outcome_is_pm_lane_non_pm_refused_1760() {
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_edit_outcome(
        &args(700, Some("Y"), None),
        "dev",
        Some(&refresh),
        &dispatch,
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
        StoryKind::Feature,
        &["enhancement"],
    )])
    .await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_edit_outcome(&args(700, Some("Y"), None), "pm", Some(&refresh), &dispatch)
        .await
        .expect_err("outcome edit aimed at a non-outcome is refused");
    assert!(
        err.contains("not an Outcome"),
        "the error teaches the boundary instead of editing a plain story: {err}"
    );
}

#[tokio::test]
async fn edit_outcome_with_no_change_is_rejected_1760() {
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_edit_outcome(&args(700, None, None), "pm", Some(&refresh), &dispatch)
        .await
        .expect_err("editing with neither --title nor --body is a no-op request");
    assert!(
        err.contains("--title") && err.contains("--body"),
        "the error names the fix: {err}"
    );
}
