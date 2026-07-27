#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use source_sync::{SourceAdapter, SyncError};

use super::live::{SwappableSource, TrackerCell};

#[tokio::test]
async fn every_clone_reads_the_one_cell() {
    let cell = TrackerCell::seeded(None);
    let held_by_a_door = cell.clone();
    let held_by_a_poller = cell.clone();
    assert!(!held_by_a_poller.is_connected());
    let adapter = super::GitHubSourceAdapter::from_connection(
        &super::connection::GithubConnection::WithBaseUrl {
            owner: "acme".to_string(),
            repo: "bpmux".to_string(),
            base_url: "http://127.0.0.1:9012/".to_string(),
        },
        cell.observer(),
    )
    .map(Arc::new)
    .expect("base_url adapter builds");
    held_by_a_door.connect(adapter);
    assert!(
        held_by_a_poller.is_connected(),
        "one write through any handle is visible to every holder — a single source, not a passed-around value"
    );
    held_by_a_door.disconnect();
    assert!(held_by_a_poller.current().is_none());
}

#[tokio::test]
async fn an_unconnected_source_fails_as_unreachable_never_as_empty_truth() {
    let source = SwappableSource::new(TrackerCell::seeded(None));
    let result = source.ingest_metadata().await;
    assert!(
        matches!(result, Err(SyncError::SourceUnreachable(_))),
        "unconnected must read as unreachable — an Ok(empty) would let a reconcile mistake absence for an empty repo"
    );
}
