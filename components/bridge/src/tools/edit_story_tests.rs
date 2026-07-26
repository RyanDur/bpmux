#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use crate::tools_stories::handle_edit_story;
use chrono::{TimeZone, Utc};
use integrations::in_memory::InMemorySourceAdapter;
use product_management::story::{Story, StoryId, Work};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use store::Dispatch;
use sync::SyncOrchestrator;

fn dispatch() -> Dispatch {
    Dispatch::recording().0
}

fn seed_story(id: u64, title: &str, body: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::Ready,
        Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.body = Some(body.to_string());
    s
}

async fn build_refresh(
    stories: Vec<Story>,
) -> (
    store_test_support::SeededStore,
    Arc<SyncOrchestrator>,
    store::Select,
) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    let select = store.select();
    (store, orch, select)
}

fn args_obj(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

#[tokio::test]
async fn edit_story_title_only_updates_title_preserves_body() {
    let (_store, refresh, select) = build_refresh(vec![seed_story(
        1234,
        "[Feature] original title",
        "## What\n\nthe body content",
    )])
    .await;
    let args = args_obj(json!({
        "id": 1234,
        "title": "[Feature] corrected title"
    }));
    let result = handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect("edit_story with title-only succeeds");
    assert!(result.contains("title"), "result mentions title: {result}");
    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(1234)))
        .expect("story still exists");
    assert_eq!(story.title, "[Feature] corrected title", "title updated");
    assert_eq!(
        story.body.as_deref(),
        Some("## What\n\nthe body content"),
        "body preserved"
    );
}

#[tokio::test]
async fn edit_story_body_only_updates_body_preserves_title() {
    let (_store, refresh, select) = build_refresh(vec![seed_story(
        1234,
        "[Feature] original title",
        "old body",
    )])
    .await;
    let args = args_obj(json!({
        "id": 1234,
        "body": "## What\n\nnew body content"
    }));
    let result = handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect("edit_story with body-only succeeds");
    assert!(result.contains("body"), "result mentions body: {result}");
    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(1234)))
        .expect("story still exists");
    assert_eq!(story.title, "[Feature] original title", "title preserved");
    assert_eq!(
        story.body.as_deref(),
        Some("## What\n\nnew body content"),
        "body updated"
    );
}

#[tokio::test]
async fn edit_story_with_both_title_and_body_updates_both() {
    let (_store, refresh, select) =
        build_refresh(vec![seed_story(1234, "old title", "old body")]).await;
    let args = args_obj(json!({
        "id": 1234,
        "title": "new title",
        "body": "new body"
    }));
    let result = handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect("edit_story with both succeeds");
    assert!(
        result.contains("title") && result.contains("body"),
        "result mentions both: {result}"
    );
    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(1234)))
        .expect("story still exists");
    assert_eq!(story.title, "new title", "title updated");
    assert_eq!(story.body.as_deref(), Some("new body"), "body updated");
}

#[tokio::test]
async fn edit_story_body_returns_optimistically_and_pushes_via_the_drain() {
    use product_management::story::events::StoryEventKind;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1234, "[Feature] t", "old body")]);
    let store = store_test_support::SeededStore::new(vec![], vec![], vec![]);
    let orch: Arc<SyncOrchestrator> = store.build_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    );
    orch.refresh_all().await.unwrap();
    let select = store.select();

    let args = args_obj(json!({ "id": 1234, "body": "## What\n\nnew body content" }));
    handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orch),
        &select,
        &dispatch(),
    )
    .await
    .expect("edit_story body succeeds");

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1234)
            ))
            .unwrap()
            .body
            .as_deref(),
        Some("## What\n\nnew body content"),
        "the store reflects the edit optimistically"
    );
    assert!(
        adapter.pushed_events().is_empty(),
        "the body edit returns optimistically — the GitHub push is deferred to the async drain: {:?}",
        adapter.pushed_events()
    );

    orch.drain_pass().await.unwrap();

    assert!(
        adapter.pushed_events().iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::BodyUpdated { body, .. }
                if e.id == StoryId::new(1234) && body == "## What\n\nnew body content"
        )),
        "the async drain pushes BodyUpdated to the adapter: {:?}",
        adapter.pushed_events()
    );
}

#[tokio::test]
async fn edit_story_acceptor_resets_and_clears_the_stale_acceptor() {
    use product_management::story::Acceptor;
    use product_management::story::events::EventMeta;
    use product_management::story::events::{StoryEvent, StoryEventKind};

    let (_store, refresh, select) = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let meta = EventMeta {
        actor: "pm".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
    };
    refresh
        .mutate(StoryEvent::new(
            meta,
            StoryId::new(1234),
            StoryEventKind::AcceptorAssigned {
                acceptor: Acceptor::Dev,
            },
        ))
        .unwrap();
    refresh.drain_pass().await.unwrap();
    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1234)
            ))
            .unwrap()
            .acceptor(),
        Some(Acceptor::Dev),
        "precondition: the create-derived acceptor is dev"
    );

    let args = args_obj(json!({ "id": 1234, "acceptor": "owner" }));
    handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect("edit_story with acceptor-only succeeds");

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1234)
            ))
            .unwrap()
            .acceptor(),
        Some(Acceptor::Owner),
        "edit --acceptor owner resets the typed acceptor to owner; the single-acceptor invariant is held by the Option<Acceptor> field, not stale labels"
    );
}

#[tokio::test]
async fn edit_story_with_neither_field_rejects() {
    let (_store, refresh, select) = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let args = args_obj(json!({ "id": 1234 }));
    let err = handle_edit_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect_err("edit_story with neither body, title, nor acceptor rejected");
    assert!(
        err.contains("at least one of 'body', 'title', or 'acceptor'"),
        "error names the required-one-of constraint: {err}"
    );
}

#[tokio::test]
async fn edit_story_rejects_non_pm_role() {
    let (_store, refresh, select) = build_refresh(vec![seed_story(1234, "title", "body")]).await;
    let args = args_obj(json!({
        "id": 1234,
        "title": "new title"
    }));
    let err = handle_edit_story(
        &args,
        "dev:claude-code",
        "dev",
        None,
        Some(&refresh),
        &select,
        &dispatch(),
    )
    .await
    .expect_err("dev role rejected from PM-only tool");
    assert!(
        err.to_lowercase().contains("pm") || err.to_lowercase().contains("role"),
        "error explains role-gating: {err}"
    );
}
