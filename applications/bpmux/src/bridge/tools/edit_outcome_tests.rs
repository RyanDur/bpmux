#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::handle_edit_outcome;
use crate::product_management::board::OutcomeReader;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::github::decode_outcome_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::outcome::{Outcome, OutcomeAction, OutcomeId};
use crate::product_management::story::{BoardState, Horizon, Story, StoryId, StoryKind};
use crate::store::{AppAction, Dispatch};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

struct SeededOutcomes(Vec<Outcome>);
impl OutcomeReader for SeededOutcomes {
    fn outcomes(&self) -> Vec<Outcome> {
        self.0.clone()
    }
}

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    (Dispatch::new(tx), rx)
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

fn seed(id: u64, title: &str, story_type: StoryKind, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
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

fn seed_outcome(id: u64, title: &str, labels: &[&str]) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = labels.iter().map(ToString::to_string).collect();
    s
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    let seeded: Vec<Outcome> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("outcome"))
        })
        .map(decode_outcome_from_story)
        .collect();
    orch.bind_outcome_reader(Arc::new(SeededOutcomes(seeded)));
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
        "pm:claude-code",
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
        "pm:claude-code",
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
        "dev:claude-code",
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
    let err = handle_edit_outcome(
        &args(700, Some("Y"), None),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
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
    let refresh = build_refresh(vec![seed_outcome(700, "X", &["outcome", "horizon.now"])]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_edit_outcome(
        &args(700, None, None),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect_err("editing with neither --title nor --body is a no-op request");
    assert!(
        err.contains("--title") && err.contains("--body"),
        "the error names the fix: {err}"
    );
}
