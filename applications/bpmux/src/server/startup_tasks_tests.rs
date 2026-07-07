#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::sync::Arc;

use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::sync::{RefreshDriver, SyncOrchestrator};

use super::startup_tasks::spawn_background_ingest;

#[tokio::test]
async fn ingest_runs_in_background_not_inline() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(7),
        "seven".to_string(),
        BoardState::NotStarted,
        chrono::Utc::now(),
        "test".to_string(),
    )]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);

    spawn_background_ingest(orch.clone());

    assert!(
        orch.all_stories().is_empty(),
        "ingest must be deferred to the background, not run inline before the server binds"
    );

    for _ in 0..100 {
        if !orch.all_stories().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert_eq!(
        orch.all_stories().len(),
        1,
        "background ingest should populate the store from the source once it runs"
    );
}
