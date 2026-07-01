#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

use crate::bridge::tools_stories::handle_create_iteration;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::github::decode_iteration_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::iteration::{Iteration, IterationAction};
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::store::{AppAction, Dispatch, Select};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

fn dispatch_channel() -> (Dispatch, UnboundedReceiver<AppAction>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppAction>();
    (Dispatch::new(tx), rx)
}

fn created(rx: &mut UnboundedReceiver<AppAction>) -> Iteration {
    let mut found = None;
    while let Ok(action) = rx.try_recv() {
        if let AppAction::Iteration(IterationAction::Upserted(iterations)) = action {
            found = iterations.into_iter().next_back();
        }
    }
    found.expect("create_iteration dispatches the new iteration onto the slice")
}

fn seed_iteration_story(id: u64, name: &str, is_closed: bool) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        BoardState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 10, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.is_closed = is_closed;
    s.raw_labels = vec!["iteration".to_string()];
    s
}

async fn build_refresh_with_stories(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    orch.bind_select(Select::seeded(vec![], iterations, vec![]));
    orch
}

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    build_refresh_with_stories(vec![]).await
}

fn args(name: &str) -> Map<String, Value> {
    let map = json!({ "name": name });
    map.as_object().cloned().unwrap_or_default()
}

#[tokio::test]
async fn create_iteration_creates_issue_with_iteration_type_and_named_title() {
    let refresh = build_empty_refresh().await;
    let (dispatch, mut rx) = dispatch_channel();
    let result =
        handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh), &dispatch)
            .await
            .expect("create_iteration succeeds");
    assert!(
        result.contains("iteration created:"),
        "result message names iteration-creation: {result}"
    );
    assert!(
        result.contains("Iteration Aquila"),
        "title includes name: {result}"
    );
    assert_eq!(
        created(&mut rx).title,
        "Iteration Aquila",
        "the created iteration is dispatched onto the typed iteration slice with its named title"
    );
}

#[tokio::test]
async fn create_iteration_rejects_duplicate_open_name() {
    let refresh = build_refresh_with_stories(vec![seed_iteration_story(42, "Aquila", false)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh), &dispatch)
        .await
        .expect_err("duplicate Aquila rejected");
    assert!(
        err.contains("'Aquila'"),
        "error names the conflicting name: {err}"
    );
    assert!(
        err.contains("#42"),
        "error names the existing iteration's number: {err}"
    );
}

#[tokio::test]
async fn create_iteration_accepts_same_name_after_previous_closed() {
    let refresh = build_refresh_with_stories(vec![seed_iteration_story(42, "Aquila", true)]).await;
    let (dispatch, _rx) = dispatch_channel();
    let result =
        handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh), &dispatch)
            .await
            .expect("re-using a closed name succeeds");
    assert!(
        result.contains("iteration created:"),
        "new iteration created after previous Aquila closed: {result}"
    );
}

#[tokio::test]
async fn create_iteration_body_reserves_member_stories_section() {
    let refresh = build_empty_refresh().await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_create_iteration(
        &args("Pegasus"),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .unwrap();
    let body = created(&mut rx).body.unwrap_or_default();
    assert!(
        body.contains("## Member stories"),
        "body reserves member-stories section: {body}"
    );
    assert!(
        body.contains("Iteration Pegasus"),
        "body header names the iteration: {body}"
    );
    assert!(
        body.contains("close_iteration"),
        "body notes close-iteration ceremony: {body}"
    );
}

#[tokio::test]
async fn create_iteration_attaches_iteration_label_atomically_at_source() {
    let refresh = build_empty_refresh().await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh), &dispatch)
        .await
        .expect("create_iteration succeeds");
    let id = created(&mut rx).id.value();
    refresh
        .refresh_story(id)
        .await
        .expect("refetch from source succeeds");
    let refetched = refresh
        .store()
        .backing_story(StoryId::new(id))
        .expect("the issue is present after a source refetch");
    assert!(
        refetched
            .raw_labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case("iteration")),
        "freshly-created iteration carries the 'iteration' label at the SOURCE atomically (not only in the synchronous local StoryCreated event), so a refetch before any TagAdded drain finds it at source: {:?}",
        refetched.raw_labels
    );
}

#[tokio::test]
async fn create_iteration_assigns_order_at_birth_1735() {
    let refresh = build_empty_refresh().await;
    let (dispatch, mut rx) = dispatch_channel();
    handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh), &dispatch)
        .await
        .expect("create_iteration succeeds");
    let iteration = created(&mut rx);
    assert!(
        iteration.iteration_order.is_some(),
        "a freshly-created iteration is born with a canonical order (no order-less iteration → no Fornax-last); got {:?}",
        iteration.iteration_order
    );
}

#[tokio::test]
async fn create_iteration_appends_order_after_existing_1735() {
    let mut existing = seed_iteration_story(42, "Aquila", false);
    existing.raw_labels = vec!["iteration".to_string(), "iteration-order.100".to_string()];
    let refresh = build_refresh_with_stories(vec![existing]).await;
    let aquila_order = refresh
        .iterations()
        .iter()
        .find(|it| it.title == "Iteration Aquila")
        .and_then(|it| it.iteration_order)
        .expect("Aquila has a seeded order");
    let (dispatch, mut rx) = dispatch_channel();
    handle_create_iteration(
        &args("Pegasus"),
        "pm:claude-code",
        Some(&refresh),
        &dispatch,
    )
    .await
    .expect("create succeeds");
    let pegasus_order = created(&mut rx).iteration_order.expect("Pegasus has order");
    assert!(
        pegasus_order > aquila_order,
        "new iteration appends AFTER existing in canonical order (Pegasus {pegasus_order} > Aquila {aquila_order})"
    );
}

#[tokio::test]
async fn create_iteration_rejects_non_pm_role() {
    let refresh = build_empty_refresh().await;
    let (dispatch, _rx) = dispatch_channel();
    let err = handle_create_iteration(&args("Lyra"), "dev:claude-code", Some(&refresh), &dispatch)
        .await
        .expect_err("dev role rejected from PM-only tool");
    assert!(
        err.contains("pm-lane"),
        "error explains PM-lane gating: {err}"
    );
}

#[tokio::test]
async fn create_iteration_requires_name_arg() {
    let refresh = build_empty_refresh().await;
    let (dispatch, _rx) = dispatch_channel();
    let empty: Map<String, Value> = Map::new();
    let err = handle_create_iteration(&empty, "pm:claude-code", Some(&refresh), &dispatch)
        .await
        .expect_err("missing name rejected");
    assert!(
        err.contains("name is required"),
        "error names the required field: {err}"
    );
}
