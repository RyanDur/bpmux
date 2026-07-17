use std::sync::Arc;

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::build_seeded_orch;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{StoryId, Work};

#[tokio::test]
async fn drain_pending_via_refresh_driver_drains_the_pending_queue() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(1, Work::NotStarted)]);
    let (orchestrator, _select) = build_seeded_orch(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        vec![],
        vec![],
        vec![],
    );
    orchestrator.refresh_all().await.unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();
    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(drained, 1);
    assert_eq!(adapter.pushed_events().len(), 1);
}
