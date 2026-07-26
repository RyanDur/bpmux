#![allow(unused_imports)]

use super::super::tools_stories::*;
use product_management::story::StoryId;
use protocol::ClientMessage;

#[tokio::test]
async fn comment_on_story_enqueues_event_via_orchestrator() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(123),
        "t".into(),
        Work::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(123));
    args.insert(
        "body".into(),
        serde_json::json!("new comment via orchestrator"),
    );
    let result =
        handle_comment_on_story(&args, "dev:claude-code", None, Some(&orchestrator), &select).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(123)))
        .unwrap();
    let comments = story.free_comments.expect("FreeCommentAppended applied");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "new comment via orchestrator");
    assert_eq!(comments[0].author, "dev");

    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(drained, 1, "event must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::FreeCommentAppended { .. } if pushed[0].id == StoryId::new(123)
    ));
}

#[tokio::test]
async fn label_story_enqueues_tag_added_event() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Tag, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(200),
        "t".into(),
        Work::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(200));
    args.insert("label".into(), serde_json::json!("desk-check-fail"));
    args.insert("operation".into(), serde_json::json!("add"));
    let result = handle_label_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(200)))
        .unwrap();
    assert_eq!(
        story.tags(),
        [Tag::DeskCheckFail],
        "the desk-check-fail label decodes to a typed Tag on the story field, not a raw label round-trip"
    );

    assert!(
        adapter.pushed_events().is_empty(),
        "label_story returns optimistically — the GH push is deferred to the async GithubDrain reconciler, so nothing is pushed before the handler returns: {:?}",
        adapter.pushed_events()
    );

    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(
        drained, 1,
        "the deferred TagSet reaches the adapter on the reconciler's drain pass"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::TagSet { tag, .. } if pushed[0].id == StoryId::new(200) && *tag == Tag::DeskCheckFail
    ));
}

#[tokio::test]
async fn label_story_enqueues_tag_removed_event() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Tag, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(200),
        "t".into(),
        Work::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut add_args = serde_json::Map::new();
    add_args.insert("id".into(), serde_json::json!(200));
    add_args.insert("label".into(), serde_json::json!("desk-check-fail"));
    add_args.insert("operation".into(), serde_json::json!("add"));
    handle_label_story(
        &add_args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
    )
    .await
    .unwrap();

    let mut remove_args = serde_json::Map::new();
    remove_args.insert("id".into(), serde_json::json!(200));
    remove_args.insert("label".into(), serde_json::json!("desk-check-fail"));
    remove_args.insert("operation".into(), serde_json::json!("remove"));
    let result = handle_label_story(
        &remove_args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(
        result.is_ok(),
        "migrated remove path should succeed: {result:?}"
    );

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(200)))
        .unwrap();
    assert!(
        story.tags().is_empty(),
        "tags should be empty after the typed unset"
    );

    assert!(
        adapter.pushed_events().is_empty(),
        "both label ops return optimistically — nothing is pushed before the reconciler drains: {:?}",
        adapter.pushed_events()
    );
    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(
        drained, 2,
        "the deferred TagSet + TagUnset both reach the adapter on one reconciler drain pass — TagSet/TagUnset do not coalesce (only iteration-class TagAdded/TagRemoved do)"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::TagSet { tag, .. } if pushed[0].id == StoryId::new(200) && *tag == Tag::DeskCheckFail
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::TagUnset { tag, .. } if pushed[1].id == StoryId::new(200) && *tag == Tag::DeskCheckFail
    ));
}

#[tokio::test]
async fn pass_desk_check_emits_gate_and_state_events() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Stage, Story, Verdict, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(201),
        "t".into(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    store.dispatch(store::AppAction::Ci(store::CiAction::Updated(
        store::CiSnapshot {
            state: ci::CiState::Green,
            run_id: None,
            head_sha: Some("test-fixture-sha".to_string()),
            display_title: None,
            conclusion: "success",
            workflows: Vec::new(),
        },
    )));
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(201));
    args.insert("acceptor".into(), serde_json::json!("owner"));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    let result = handle_pass_desk_check(
        &args,
        "designer:claude-code",
        "designer",
        None,
        Some(&orchestrator),
        &select,
        None,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(201)))
        .unwrap();
    assert_eq!(story.state, Work::Acceptance);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Succeed {
                step: "desk-check".into()
            }),
        "verdicts should contain PassDeskCheck entry: {gates:?}"
    );

    orchestrator.drain_pass().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "acceptor + gate + state events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::AcceptorAssigned {
            acceptor, ..
        } if pushed[0].id == StoryId::new(201) && *acceptor == product_management::story::Acceptor::Owner
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed { step },
            payload: None,
            ..
        } if pushed[1].id == StoryId::new(201) && step.as_str() == "desk-check"
    ));
    assert!(matches!(
        &pushed[2].kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::Acceptance,
            ..
        } if pushed[2].id == StoryId::new(201)
    ));
    let labels = adapter.labels_for(201);
    assert!(
        !labels.iter().any(|l| l == "desk-check") && labels.iter().any(|l| l == "delivered"),
        "the transition export strips the stale desk-check label and applies delivered at the adapter; got {labels:?}"
    );
}

#[tokio::test]
async fn fail_desk_check_emits_gate_with_reason_and_state_events() {
    use chrono::{TimeZone, Utc};
    use integrations::in_memory::InMemorySourceAdapter;
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Stage, Story, Verdict, Work};
    use std::sync::Arc;
    use sync::SyncOrchestrator;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(202),
        "t".into(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(202, vec!["desk-check".to_string()]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orchestrator: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(202));
    let substantive_reason = "observed: ATP file foo.rs has zero assertions, refutes clause (b)";
    args.insert("reason".into(), serde_json::json!(substantive_reason));
    let result = handle_fail_desk_check(
        &args,
        "designer:claude-code",
        "designer",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(202)))
        .unwrap();
    assert_eq!(story.state, Work::InProgress);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Fail {
                step: "desk-check".into()
            }
            && g.payload.as_deref() == Some(substantive_reason)),
        "verdicts should contain FailDeskCheck with payload: {gates:?}"
    );

    orchestrator.drain_pass().await.unwrap();
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
        &pushed[0].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Fail { step },
            payload: Some(p),
            ..
        } if pushed[0].id == StoryId::new(202) && p == substantive_reason && step.as_str() == "desk-check"
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            ..
        } if pushed[1].id == StoryId::new(202)
    ));
}
