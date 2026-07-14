#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, SeededStore};
use crate::sync::{RefreshDriver, SyncOrchestrator};
use chrono::{TimeZone, Utc};
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId, StoryKind, Work};

fn meta() -> EventMeta {
    EventMeta {
        actor: "pm".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 7, 3, 0, 0, 0).unwrap(),
    }
}

fn bound_orchestrator() -> (SeededStore, Arc<dyn RefreshDriver>) {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let handle = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    handle.bind_refresh(&orch);
    (handle, orch)
}

fn created(id: u64, title: &str) -> StoryEvent {
    StoryEvent::new(
        meta(),
        StoryId::new(id),
        StoryEventKind::StoryCreated {
            title: title.to_string(),
            body: "b".to_string(),
            initial_state: Work::NotStarted,
            story_type: StoryKind::Feature,
        },
    )
}

fn minimal_story(id: u64, title: &str) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 3, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[test]
fn an_orchestrator_write_is_synchronously_visible_through_its_own_reads() {
    let (_handle, orch) = bound_orchestrator();
    orch.mutate_sync(created(42, "written")).unwrap();
    let read_back = orch.story(42).expect(
        "the story is readable the moment mutate_sync returns \
         (synchronous dispatch — read-your-write holds)",
    );
    assert_eq!(read_back.title, "written");
    assert_eq!(read_back.state, Work::NotStarted);
}

#[test]
fn generic_tag_churn_has_no_domain_side_effect_after_the_middleware_retire() {
    let (_handle, orch) = bound_orchestrator();
    orch.mutate_sync(created(7, "labeled")).unwrap();
    orch.mutate_sync(StoryEvent::new(
        meta(),
        StoryId::new(7),
        StoryEventKind::TagAdded {
            label: "some-generic-label".to_string(),
        },
    ))
    .unwrap();
    let story = orch.story(7).unwrap();
    assert!(
        story.tags().is_empty(),
        "a generic TagAdded is edge-bound only — no reducer arm and no middleware \
         projects it into domain state; it reaches GitHub via the outbox and returns \
         typed through the adapter refresh"
    );
}

#[test]
fn seed_remove_and_clear_act_on_the_bound_central_store() {
    let (handle, orch) = bound_orchestrator();
    assert!(
        orch.store_is_empty(),
        "a freshly bound central store starts empty"
    );
    let seeded = minimal_story(9, "seeded");
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seeded.clone()])));
    assert_eq!(orch.story(9), Some(seeded.clone()));
    assert!(
        orch.remove_story(404).is_none(),
        "removing an absent story yields nothing"
    );
    let removed = orch.remove_story(9);
    assert_eq!(
        removed,
        Some(seeded),
        "remove_story returns the story it removed"
    );
    assert!(
        orch.story(9).is_none(),
        "the removed story is gone from reads"
    );
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![minimal_story(
        10,
        "for-clear",
    )])));
    orch.clear_stories();
    assert!(
        orch.store_is_empty(),
        "clear_stories empties the bound store"
    );
}

#[test]
fn an_unbound_orchestrator_drops_writes_and_reads_come_back_empty() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    orch.mutate_sync(created(1, "dropped")).unwrap();
    assert!(
        orch.story(1).is_none(),
        "an unbound write is dropped — reads stay empty instead of panicking"
    );
    assert!(orch.all_stories().is_empty());
    assert!(orch.store_is_empty());
}
