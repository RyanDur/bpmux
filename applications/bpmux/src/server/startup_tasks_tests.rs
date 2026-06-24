#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::sync::Arc;

use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::domain::{Story, StoryState};
use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};

use super::spawn_background_ingest;

#[tokio::test]
async fn ingest_runs_in_background_not_inline() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        7,
        "seven".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "test".to_string(),
    )]);
    let store = Arc::new(BoardStore::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store.clone(), adapter));

    spawn_background_ingest(orch.clone());

    assert!(
        store.all_stories().is_empty(),
        "ingest must be deferred to the background, not run inline before the server binds"
    );

    for _ in 0..100 {
        if !store.all_stories().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert_eq!(
        store.all_stories().len(),
        1,
        "background ingest should populate the store from the source once it runs"
    );
}
