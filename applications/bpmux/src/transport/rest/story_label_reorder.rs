#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::labels::{LabelMutationPayload, post_story_labels};
use crate::product_management::board::adapters::github::iteration_membership_from_labels;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{BoardStore, RefreshDriver, StoreChange, SyncOrchestrator};
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{Story, StoryId, StoryState};
use crate::session::Session;
use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::broadcast;

fn membered_member(id: u64, label: &str) -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(id),
        "reorder fixture".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "tester".to_string(),
    );
    story.raw_labels = vec![label.to_string()];
    story.iteration_membership = iteration_membership_from_labels(&story.raw_labels)
        .map(|(id, position)| (IterationId::new(id), position));
    story
}

fn membership(store: &BoardStore, id: u64) -> Option<(IterationId, u32)> {
    store
        .story(StoryId::new(id))
        .unwrap()
        .iteration_membership()
}

#[tokio::test]
async fn reordering_a_member_through_the_label_endpoint_never_transiently_orphans_it_1524() {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![membered_member(1687, "parent-iteration-#1655.125")]);
    let refresh: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&store), adapter));
    refresh.refresh_all().await.unwrap();

    let mut emitted = store.subscribe();

    let dir = tempfile::tempdir().unwrap();
    let (wake_tx, _wake_rx) = broadcast::channel(16);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));

    let payload = LabelMutationPayload {
        add: vec!["parent-iteration-#1655.106".to_string()],
        remove: vec!["parent-iteration-#1655.125".to_string()],
    };
    let response = post_story_labels(
        State(session),
        axum::Extension(Some(Arc::clone(&refresh))),
        Path(1687),
        Json(payload),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let mut sequence = Vec::new();
    while let Ok(change) = emitted.try_recv() {
        if let StoreChange::Event(event) = change {
            sequence.push(event);
        }
    }

    let replay = BoardStore::new();
    replay.seed_all(vec![membered_member(1687, "parent-iteration-#1655.125")]);
    assert!(
        membership(&replay, 1687).is_some(),
        "fixture starts membered"
    );
    for event in &sequence {
        replay.apply(event);
        assert!(
            membership(&replay, 1687).is_some(),
            "a reorder must never transiently orphan the story (#1524): the label-mutation \
             endpoint must emit the ADD of the new parent-iteration label before the REMOVE of \
             the old one, so a concurrent board read never drops the row from its iteration until \
             refresh; the emitted sequence was {sequence:?}"
        );
    }

    assert_eq!(
        membership(&store, 1687),
        Some((IterationId::new(1655), 106)),
        "after the swap the story is membered at the new position"
    );
}
