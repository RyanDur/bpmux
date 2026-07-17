#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::sync::Arc;

use crate::integrations::in_memory::InMemorySourceAdapter;
use product_management::story::{Story, StoryId, Work};

use super::startup_tasks::spawn_background_ingest;

#[tokio::test]
async fn ingest_runs_in_background_not_inline() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(7),
        "seven".to_string(),
        Work::NotStarted,
        chrono::Utc::now(),
        "test".to_string(),
    )]);
    let (orch, select) = crate::store::build_seeded_orch(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        vec![],
        vec![],
        vec![],
    );
    let stories_in_store =
        || select.select(|state| product_management::story::slice::all(&state.stories));

    spawn_background_ingest(
        orch.clone(),
        crate::store::Dispatch::recording().0,
        select.clone(),
    );

    assert!(
        stories_in_store().is_empty(),
        "ingest must be deferred to the background, not run inline before the server binds"
    );

    for _ in 0..100 {
        if !stories_in_store().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    assert_eq!(
        stories_in_store().len(),
        1,
        "background ingest should populate the store from the source once it runs"
    );
}
