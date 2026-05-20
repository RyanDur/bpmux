#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::{
    connected_ws_sink_for_test, connected_ws_sink_with_capture, test_ci_state_store_green,
};
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn comment_on_story_migrated_path_enqueues_event_via_orchestrator() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        123,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(123));
    args.insert(
        "body".into(),
        serde_json::json!("new comment via orchestrator"),
    );
    let result =
        handle_comment_on_story(&args, "dev:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(123).unwrap();
    let comments = story.free_comments.expect("FreeCommentAppended applied");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "new comment via orchestrator");
    assert_eq!(comments[0].author, "dev");

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "event must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::FreeCommentAppended { number: 123, .. }
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_added_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, Tag};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        200,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(200));
    args.insert("label".into(), serde_json::json!("blocked"));
    args.insert("operation".into(), serde_json::json!("add"));
    let result = handle_label_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(200).unwrap();
    assert_eq!(story.tags, vec![Tag::Blocked]);

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagAdded { number: 200, label, .. } if label == "blocked"
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_removed_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        200,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);

    let mut add_args = serde_json::Map::new();
    add_args.insert("number".into(), serde_json::json!(200));
    add_args.insert("label".into(), serde_json::json!("blocked"));
    add_args.insert("operation".into(), serde_json::json!("add"));
    handle_label_story(&add_args, "pm:claude-code", &ws_tx, Some(&orchestrator))
        .await
        .unwrap();

    let mut remove_args = serde_json::Map::new();
    remove_args.insert("number".into(), serde_json::json!(200));
    remove_args.insert("label".into(), serde_json::json!("blocked"));
    remove_args.insert("operation".into(), serde_json::json!("remove"));
    let result =
        handle_label_story(&remove_args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "migrated remove path should succeed: {result:?}"
    );

    let story = store.story(200).unwrap();
    assert!(story.tags.is_empty(), "tags should be empty after remove");

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally per add + remove; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagAdded { number: 200, label, .. } if label == "blocked"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::TagRemoved { number: 200, label, .. } if label == "blocked"
    ));
}

#[tokio::test]
async fn pass_desk_check_migrated_path_emits_gate_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        201,
        "t".into(),
        StoryState::DeskCheck,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orch_concrete = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
        .with_ci_state_store(test_ci_state_store_green());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(201));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    let result = handle_pass_desk_check(
        &args,
        "designer:claude-code",
        &ws_tx,
        Some(&orchestrator),
        None,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(201).unwrap();
    assert_eq!(story.state, StoryState::Delivered);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::PassDeskCheck),
        "gate_comments should contain PassDeskCheck entry: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3, "3 events expected post-#1156: {pushed:?}");
    let reap_labels: Vec<&str> = pushed[..1]
        .iter()
        .filter_map(|e| match e {
            BoardEvent::TagRemoved {
                number: 201, label, ..
            } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reap_labels,
        vec!["desk-check"],
        "post-#1156 pre-filter: only present-stale 'desk-check' label reaped"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 201,
            gate: Gate::PassDeskCheck,
            payload: None,
            ..
        }
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 201,
            new_state: StoryState::Delivered,
            ..
        }
    ));
}

#[tokio::test]
async fn fail_desk_check_migrated_path_emits_gate_with_reason_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        202,
        "t".into(),
        StoryState::DeskCheck,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(202));
    let substantive_reason = "observed: ATP file foo.rs has zero assertions, refutes clause (b)";
    args.insert("reason".into(), serde_json::json!(substantive_reason));
    let result =
        handle_fail_desk_check(&args, "designer:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(202).unwrap();
    assert_eq!(story.state, StoryState::InProgress);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::FailDeskCheck
                && g.payload.as_deref() == Some(substantive_reason)),
        "gate_comments should contain FailDeskCheck with payload: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3, "3 events expected post-#1156: {pushed:?}");
    let reap_labels: Vec<&str> = pushed[..1]
        .iter()
        .filter_map(|e| match e {
            BoardEvent::TagRemoved {
                number: 202, label, ..
            } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reap_labels,
        vec!["desk-check"],
        "post-#1156 pre-filter: only present-stale 'desk-check' label reaped (from-state)"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 202,
            gate: Gate::FailDeskCheck,
            payload: Some(p),
            ..
        } if p == substantive_reason
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 202,
            new_state: StoryState::InProgress,
            ..
        }
    ));
}
