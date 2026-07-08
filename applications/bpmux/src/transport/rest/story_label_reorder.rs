#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::labels::{LabelMutationPayload, post_story_labels};
use crate::hub::Hub;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{Story, StoryId, Work};
use crate::sync::{RefreshDriver, SyncOrchestrator};
use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::broadcast;

fn membered_member(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        "reorder fixture".to_string(),
        Work::NotStarted,
        chrono::Utc::now(),
        "tester".to_string(),
    )
}

#[tokio::test]
async fn reordering_a_member_through_the_label_endpoint_never_orphans_it_1524() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![membered_member(1687)]);
    adapter.seed_labels(1687, vec!["parent-iteration-#1655.125".to_string()]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&refresh, vec![], vec![], vec![]);
    refresh.refresh_all().await.unwrap();
    assert!(
        adapter
            .labels_for(1687)
            .iter()
            .any(|l| l.starts_with("parent-iteration-#")),
        "fixture starts membered"
    );

    let dir = tempfile::tempdir().unwrap();
    let (wake_tx, _wake_rx) = broadcast::channel(16);
    let hub = Arc::new(Hub::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        wake_tx,
    ));

    let payload = LabelMutationPayload {
        add: vec!["parent-iteration-#1655.106".to_string()],
        remove: vec!["parent-iteration-#1655.125".to_string()],
    };
    let (dispatch_tx, _dispatch_rx) = tokio::sync::mpsc::unbounded_channel();
    let response = post_story_labels(
        State(hub),
        axum::Extension(Some(Arc::clone(&refresh))),
        axum::Extension(crate::store::Dispatch::new(dispatch_tx)),
        Path(1687),
        Json(payload),
    )
    .await
    .into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    refresh.drain_pending().await.unwrap();

    refresh
        .story(1687)
        .expect("a reorder must never orphan the story (#1524)");
    let edge_labels = adapter.labels_for(1687);
    assert_eq!(
        edge_labels,
        vec!["parent-iteration-#1655.106".to_string()],
        "after the swap the story is membered at exactly the new position at the \
         GitHub edge — the new parent-iteration label landed and the old one is gone, \
         so no board read can drop the row from its iteration (#1524)"
    );
}
