#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(unsafe_code)]

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::bridge::tools_stories::handle_create_iteration;
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

fn seed_iteration_story(number: u64, theme: &str, is_closed: bool) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {theme}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 10, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.story_type = StoryType::Iteration;
    s.is_closed = is_closed;
    s.raw_labels = vec!["iteration".to_string()];
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

async fn build_empty_refresh() -> Arc<dyn RefreshDriver> {
    build_refresh_with_stories(vec![]).await
}

fn args(theme: &str) -> Map<String, Value> {
    let map = json!({ "theme_name": theme });
    map.as_object().cloned().unwrap_or_default()
}

#[tokio::test]
async fn create_iteration_creates_issue_with_iteration_type_and_themed_title() {
    let refresh = build_empty_refresh().await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    let result = handle_create_iteration(&args("Aquila"), "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect("create_iteration succeeds");
    assert!(
        result.contains("iteration created:"),
        "result message names iteration-creation: {result}"
    );
    assert!(
        result.contains("Iteration Aquila"),
        "title includes theme: {result}"
    );
    let stories = refresh.all_stories();
    let iteration = stories
        .iter()
        .find(|s| s.title == "Iteration Aquila")
        .expect("iteration issue persisted");
    assert_eq!(
        iteration.story_type,
        StoryType::Iteration,
        "story_type=Iteration"
    );
}

#[tokio::test]
async fn create_iteration_rejects_duplicate_open_theme_name() {
    let refresh = build_refresh_with_stories(vec![seed_iteration_story(42, "Aquila", false)]).await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    let err = handle_create_iteration(&args("Aquila"), "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect_err("duplicate Aquila rejected");
    assert!(
        err.contains("'Aquila'"),
        "error names the conflicting theme: {err}"
    );
    assert!(
        err.contains("#42"),
        "error names the existing iteration's number: {err}"
    );
}

#[tokio::test]
async fn create_iteration_accepts_same_name_after_previous_closed() {
    let refresh = build_refresh_with_stories(vec![seed_iteration_story(42, "Aquila", true)]).await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    let result = handle_create_iteration(&args("Aquila"), "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect("re-using a closed theme_name succeeds");
    assert!(
        result.contains("iteration created:"),
        "new iteration created after previous Aquila closed: {result}"
    );
}

#[tokio::test]
async fn create_iteration_body_reserves_member_stories_section() {
    let refresh = build_empty_refresh().await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    handle_create_iteration(&args("Pegasus"), "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .unwrap();
    let stories = refresh.all_stories();
    let iteration = stories
        .iter()
        .find(|s| s.title == "Iteration Pegasus")
        .expect("Pegasus issue persisted");
    let body = iteration.body.clone().unwrap_or_default();
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
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    handle_create_iteration(&args("Aquila"), "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect("create_iteration succeeds");
    let number = refresh
        .all_stories()
        .iter()
        .find(|s| s.title == "Iteration Aquila")
        .expect("iteration persisted")
        .number;
    refresh
        .refresh_story(number)
        .await
        .expect("refetch from source succeeds");
    let refetched = refresh
        .story(number)
        .expect("iteration present after source refetch");
    assert!(
        refetched
            .raw_labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case("iteration")),
        "freshly-created iteration carries the 'iteration' label at the SOURCE atomically (not only in the synchronous local StoryCreated event), so a refetch before any TagAdded drain cannot type it as non-Iteration: {:?}",
        refetched.raw_labels
    );
    assert_eq!(
        crate::board::domain::story_type_from_labels_or_title(
            &refetched.raw_labels,
            &refetched.title
        ),
        StoryType::Iteration,
        "story_type re-derived from the source-of-truth labels stays Iteration (no async window where it reads as Feature)"
    );
}

#[tokio::test]
async fn create_iteration_rejects_non_pm_role() {
    let refresh = build_empty_refresh().await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    let err = handle_create_iteration(&args("Lyra"), "dev:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect_err("dev role rejected from PM-only tool");
    assert!(
        err.contains("pm-lane"),
        "error explains PM-lane gating: {err}"
    );
}

#[tokio::test]
async fn create_iteration_requires_theme_name_arg() {
    let refresh = build_empty_refresh().await;
    let ws_tx: Mutex<Option<crate::bridge::WsSink>> = Mutex::new(None);
    let empty: Map<String, Value> = Map::new();
    let err = handle_create_iteration(&empty, "pm:claude-code", &ws_tx, Some(&refresh))
        .await
        .expect_err("missing theme_name rejected");
    assert!(
        err.contains("theme_name is required"),
        "error names the required field: {err}"
    );
}
