#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::story::StoryId;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn comment_on_story_migrated_path_enqueues_event_via_orchestrator() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::StoryEvent;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(123),
        "t".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(123));
    args.insert(
        "body".into(),
        serde_json::json!("new comment via orchestrator"),
    );
    let result = handle_comment_on_story(&args, "dev:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(StoryId::new(123)).unwrap();
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
        StoryEvent::FreeCommentAppended { id, .. } if *id == StoryId::new(123)
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_added_event() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::StoryEvent;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story, Tag};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(200),
        "t".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(200));
    args.insert("label".into(), serde_json::json!("blocked"));
    args.insert("operation".into(), serde_json::json!("add"));
    let result = handle_label_story(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(StoryId::new(200)).unwrap();
    assert_eq!(
        story.tags(),
        [Tag::Blocked],
        "the blocked label decodes to a typed Tag on the story field, not a raw label round-trip"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        StoryEvent::TagSet { id, tag, .. } if *id == StoryId::new(200) && *tag == Tag::Blocked
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_removed_event() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::StoryEvent;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story, Tag};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(200),
        "t".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut add_args = serde_json::Map::new();
    add_args.insert("id".into(), serde_json::json!(200));
    add_args.insert("label".into(), serde_json::json!("blocked"));
    add_args.insert("operation".into(), serde_json::json!("add"));
    handle_label_story(&add_args, "pm:claude-code", Some(&orchestrator))
        .await
        .unwrap();

    let mut remove_args = serde_json::Map::new();
    remove_args.insert("id".into(), serde_json::json!(200));
    remove_args.insert("label".into(), serde_json::json!("blocked"));
    remove_args.insert("operation".into(), serde_json::json!("remove"));
    let result = handle_label_story(&remove_args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "migrated remove path should succeed: {result:?}"
    );

    let story = store.story(StoryId::new(200)).unwrap();
    assert!(
        story.tags().is_empty(),
        "tags should be empty after the typed unset"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "#1142: handler drains internally per add + remove; nothing left to drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        StoryEvent::TagSet { id, tag, .. } if *id == StoryId::new(200) && *tag == Tag::Blocked
    ));
    assert!(matches!(
        &pushed[1],
        StoryEvent::TagUnset { id, tag, .. } if *id == StoryId::new(200) && *tag == Tag::Blocked
    ));
}

#[tokio::test]
async fn pass_desk_check_migrated_path_emits_gate_and_state_events() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::StoryEvent;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Gate, Stage, Story};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(201),
        "t".into(),
        BoardState::InProgress(Stage::DeskCheck),
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orch_concrete = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
        .with_ci_state_store(test_ci_state_store_green());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(201));
    args.insert("acceptor".into(), serde_json::json!("owner"));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    let result =
        handle_pass_desk_check(&args, "designer:claude-code", Some(&orchestrator), None).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(StoryId::new(201)).unwrap();
    assert_eq!(story.state, BoardState::Delivered);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::PassDeskCheck),
        "gate_comments should contain PassDeskCheck entry: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "acceptor + gate + state events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(matches!(
        &pushed[0],
        StoryEvent::AcceptorAssigned {
            id, acceptor, ..
        } if *id == StoryId::new(201) && *acceptor == crate::product_management::story::Acceptor::Owner
    ));
    assert!(matches!(
        &pushed[1],
        StoryEvent::GateCommentAppended {
            id,
            gate: Gate::PassDeskCheck,
            payload: None,
            ..
        } if *id == StoryId::new(201)
    ));
    assert!(matches!(
        &pushed[2],
        StoryEvent::StateTransitioned {
            id,
            new_state: BoardState::Delivered,
            ..
        } if *id == StoryId::new(201)
    ));
    let labels = adapter.labels_for(201);
    assert!(
        !labels.iter().any(|l| l == "desk-check") && labels.iter().any(|l| l == "delivered"),
        "the transition export strips the stale desk-check label and applies delivered at the adapter; got {labels:?}"
    );
}

#[tokio::test]
async fn fail_desk_check_migrated_path_emits_gate_with_reason_and_state_events() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::StoryEvent;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Gate, Stage, Story};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(202),
        "t".into(),
        BoardState::InProgress(Stage::DeskCheck),
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(202));
    let substantive_reason = "observed: ATP file foo.rs has zero assertions, refutes clause (b)";
    args.insert("reason".into(), serde_json::json!(substantive_reason));
    let result = handle_fail_desk_check(&args, "designer:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(StoryId::new(202)).unwrap();
    assert_eq!(story.state, BoardState::InProgress(Stage::Dev));
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::FailDeskCheck
                && g.payload.as_deref() == Some(substantive_reason)),
        "gate_comments should contain FailDeskCheck with payload: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        2,
        "gate + state events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    let labels = adapter.labels_for(202);
    assert!(
        !labels.iter().any(|l| l == "desk-check") && labels.iter().any(|l| l == "in progress"),
        "the transition export strips the stale desk-check label and applies in progress at the adapter; got {labels:?}"
    );
    assert!(matches!(
        &pushed[0],
        StoryEvent::GateCommentAppended {
            id,
            gate: Gate::FailDeskCheck,
            payload: Some(p),
            ..
        } if *id == StoryId::new(202) && p == substantive_reason
    ));
    assert!(matches!(
        &pushed[1],
        StoryEvent::StateTransitioned {
            id,
            new_state: BoardState::InProgress(Stage::Dev),
            ..
        } if *id == StoryId::new(202)
    ));
}
