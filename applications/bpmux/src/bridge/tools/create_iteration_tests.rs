#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

use crate::bridge::tools_stories::handle_create_iteration;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::story::{Story, StoryId, StoryState, StoryType};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn seed_iteration_story(number: u64, name: &str, is_closed: bool) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(number),
        format!("Iteration {name}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 10, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.is_closed = is_closed;
    s.raw_labels = vec!["iteration".to_string()];
    s.story_type = StoryType::Iteration;
    s
}

async fn build_refresh_with_stories(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch = SyncOrchestrator::new(store, adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
    orch.refresh_all().await.unwrap();
    orch
}

fn iteration_number_by_title(refresh: &Arc<dyn RefreshDriver>, title: &str) -> u64 {
    refresh
        .iterations()
        .iter()
        .find(|it| it.content.title == title)
        .and_then(|it| it.id().saved_id().map(|i| i.value()))
        .unwrap_or_else(|| panic!("iteration '{title}' persisted"))
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
    let result = handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh))
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
    let number = iteration_number_by_title(&refresh, "Iteration Aquila");
    assert!(
        refresh.iteration(number).is_some(),
        "the created issue is typed as an Iteration (present in the typed iteration collection)"
    );
}

#[tokio::test]
async fn create_iteration_rejects_duplicate_open_name() {
    let refresh = build_refresh_with_stories(vec![seed_iteration_story(42, "Aquila", false)]).await;
    let err = handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh))
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
    let result = handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh))
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
    handle_create_iteration(&args("Pegasus"), "pm:claude-code", Some(&refresh))
        .await
        .unwrap();
    let number = iteration_number_by_title(&refresh, "Iteration Pegasus");
    let iteration = refresh.iteration(number).expect("Pegasus issue persisted");
    let body = iteration.content.body.clone().unwrap_or_default();
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
    handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh))
        .await
        .expect("create_iteration succeeds");
    let number = iteration_number_by_title(&refresh, "Iteration Aquila");
    refresh
        .refresh_story(number)
        .await
        .expect("refetch from source succeeds");
    let refetched = refresh
        .store()
        .backing_story(StoryId::new(number))
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
    handle_create_iteration(&args("Aquila"), "pm:claude-code", Some(&refresh))
        .await
        .expect("create_iteration succeeds");
    let number = iteration_number_by_title(&refresh, "Iteration Aquila");
    let iteration = refresh.iteration(number).expect("iteration persisted");
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
    handle_create_iteration(&args("Pegasus"), "pm:claude-code", Some(&refresh))
        .await
        .expect("create succeeds");
    let pegasus = refresh
        .iteration(iteration_number_by_title(&refresh, "Iteration Pegasus"))
        .expect("Pegasus persisted");
    let aquila = refresh
        .iteration(iteration_number_by_title(&refresh, "Iteration Aquila"))
        .expect("Aquila persisted");
    assert!(
        pegasus.iteration_order.expect("Pegasus has order")
            > aquila.iteration_order.expect("Aquila has order"),
        "new iteration appends AFTER existing in canonical order (Pegasus {:?} > Aquila {:?})",
        pegasus.iteration_order,
        aquila.iteration_order
    );
}

#[tokio::test]
async fn create_iteration_rejects_non_pm_role() {
    let refresh = build_empty_refresh().await;
    let err = handle_create_iteration(&args("Lyra"), "dev:claude-code", Some(&refresh))
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
    let empty: Map<String, Value> = Map::new();
    let err = handle_create_iteration(&empty, "pm:claude-code", Some(&refresh))
        .await
        .expect_err("missing name rejected");
    assert!(
        err.contains("name is required"),
        "error names the required field: {err}"
    );
}
