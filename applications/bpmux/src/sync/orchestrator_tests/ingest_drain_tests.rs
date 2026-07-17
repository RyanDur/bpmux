#![allow(unused_imports)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::store::{AppAction, SeededStore};
use crate::sync::RefreshDriver;
use crate::sync::SyncOrchestrator;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Stage, Story, StoryId, StoryKind, Work};

#[tokio::test]
async fn ingest_once_populates_store_from_adapter() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![
        seed_story(1, Work::NotStarted),
        seed_story(2, Work::Acceptance),
    ]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();
    orchestrator.ingest_once().await.unwrap();
    assert_eq!(
        select
            .select(|state| product_management::story::slice::all(&state.stories))
            .len(),
        2
    );
    assert!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(1)))
            .is_some()
    );
    assert!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(2)))
            .is_some()
    );
    assert!(orchestrator.snapshot_status().last_ingest_at.is_some());
}

#[tokio::test]
async fn ingest_once_records_error_on_source_failure() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let result = orchestrator.ingest_once().await;
    assert!(result.is_err());
    let status = orchestrator.snapshot_status();
    assert!(status.last_error.is_some());
    assert!(status.last_ingest_at.is_none());
}

#[tokio::test]
async fn ingest_success_after_failure_clears_the_last_error() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);

    assert!(orchestrator.ingest_once().await.is_err());
    assert!(
        orchestrator.snapshot_status().last_error.is_some(),
        "the failed ingest records an error on the shared field"
    );

    adapter.set_ingest_failures(false);
    orchestrator.ingest_once().await.unwrap();

    let status = orchestrator.snapshot_status();
    assert!(
        status.last_error.is_none(),
        "a subsequent successful ingest clears the error the shared field carried"
    );
    assert!(status.last_ingest_at.is_some());
    assert!(!status.startup_fail);
}

#[tokio::test]
async fn mutate_applies_event_locally_and_enqueues_for_push() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(100),
            StoryEventKind::StoryCreated {
                title: "test".to_string(),
                body: "b".to_string(),
                initial_state: Work::NotStarted,
                story_type: StoryKind::Feature,
            },
        ))
        .unwrap();
    assert_eq!(
        handle
            .select()
            .select(|state| product_management::story::slice::all(&state.stories))
            .len(),
        1
    );
    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(100)
            ))
            .unwrap()
            .title,
        "test"
    );
    assert_eq!(orchestrator.snapshot_status().pending_events, 1);
    assert!(
        adapter.pushed_events().is_empty(),
        "push is async; drain_pass fires it"
    );
}

#[tokio::test]
async fn drain_pass_pushes_pending_events_to_adapter() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        1,
        Work::NotStarted,
    )])));
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
    assert_eq!(orchestrator.snapshot_status().pending_events, 0);
    assert!(orchestrator.snapshot_status().last_push_at.is_some());
}

#[tokio::test]
async fn drain_pass_requeues_event_on_adapter_failure() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        1,
        Work::NotStarted,
    )])));
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
    adapter.set_push_failures(true);
    let result = orchestrator.drain_pass().await;
    assert!(result.is_err(), "drain should surface failure count");
    assert!(
        adapter.pushed_events().is_empty(),
        "failed push does not record"
    );
    let status = orchestrator.snapshot_status();
    assert!(
        status.last_error.is_some(),
        "error recorded on drain failure"
    );
    adapter.set_push_failures(false);
    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(drained, 1, "requeued event drains on retry");
}

#[tokio::test]
async fn drain_pass_handles_multiple_events_in_order() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        seed_story(1, Work::NotStarted),
        seed_story(2, Work::NotStarted),
    ])));
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
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(2),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
    assert_eq!(orchestrator.snapshot_status().pending_events, 2);
    let drained = orchestrator.drain_pass().await.unwrap();
    assert_eq!(drained, 2);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert_eq!(pushed[0].story_id(), StoryId::new(1));
    assert_eq!(pushed[1].story_id(), StoryId::new(2));
}

#[tokio::test]
async fn ingest_preserves_an_in_flight_label_mutation_against_a_pre_drain_refresh() {
    use product_management::story::Tag;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = seed_story(1687, Work::NotStarted);
    adapter.seed_stories(vec![seeded]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator.ingest_once().await.unwrap();
    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1687)
            ))
            .unwrap()
            .tags(),
        &[] as &[Tag],
        "baseline: the source carries no tag yet"
    );

    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(1687),
            StoryEventKind::TagSet {
                tag: Tag::DeskCheckFail,
            },
        ))
        .unwrap();
    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1687)
            ))
            .unwrap()
            .tags(),
        &[Tag::DeskCheckFail],
        "optimistic label mutation lands the tag"
    );

    orchestrator.ingest_once().await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(1687)
            ))
            .unwrap()
            .tags(),
        &[Tag::DeskCheckFail],
        "a refresh that reads the source before the mutation's write has drained must NOT clobber the un-acknowledged optimistic tag"
    );
}

#[tokio::test]
async fn drain_echo_defers_while_a_later_write_for_the_same_story_is_still_queued() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        42,
        Work::NotStarted,
    )])));

    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(42),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(42),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

    adapter.queue_push_follow_up(StoryEvent::new(
        meta(),
        StoryId::new(42),
        StoryEventKind::StoryRefreshed {
            story: Box::new(seed_story(42, Work::Acceptance)),
        },
    ));

    orchestrator.drain_pass().await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(42)))
            .unwrap()
            .state,
        Work::InProgress,
        "the deliver echo carrying earlier remote truth must not clobber the still-queued reopen's optimistic state"
    );
}

#[tokio::test]
async fn drain_echo_applies_full_remote_authority_once_the_queue_for_the_story_is_empty() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();
    handle.dispatch(AppAction::Story(StoryAction::Seeded(vec![seed_story(
        43,
        Work::NotStarted,
    )])));

    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(43),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        ))
        .unwrap();

    let mut echo_story = seed_story(43, Work::Accepted);
    echo_story.title = "authoritative-remote-title".to_string();
    echo_story.remote_updated_at = Some(Utc.with_ymd_and_hms(2026, 4, 25, 16, 0, 0).unwrap());
    adapter.queue_push_follow_up(StoryEvent::new(
        meta(),
        StoryId::new(43),
        StoryEventKind::StoryRefreshed {
            story: Box::new(echo_story),
        },
    ));

    orchestrator.drain_pass().await.unwrap();

    let refreshed = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(43)))
        .unwrap();
    assert_eq!(
        refreshed.state,
        Work::Accepted,
        "with no later write queued the echo defers to nothing and remote authority applies"
    );
    assert_eq!(
        refreshed.title, "authoritative-remote-title",
        "the guard defers the echo, it never discards its authority"
    );
}

#[tokio::test]
async fn ingest_defers_a_stale_state_snapshot_while_a_local_write_is_queued() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(44, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator.ingest_once().await.unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(44),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

    orchestrator.ingest_once().await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(44)))
            .unwrap()
            .state,
        Work::InProgress,
        "a stale remote snapshot read before the queued state write drained must NOT clobber the optimistic state"
    );
}

#[tokio::test]
async fn refresh_story_defers_seed_while_a_local_write_is_queued_for_a_present_story() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(416, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator.ingest_once().await.unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(416),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();

    orchestrator.refresh_story(416).await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(
                &state.stories,
                StoryId::new(416)
            ))
            .unwrap()
            .state,
        Work::InProgress,
        "a single-story refresh that reads the source before the queued write drained must NOT clobber the optimistic state of a present story"
    );
}

#[tokio::test]
async fn refresh_story_seeds_an_absent_story_even_while_tag_writes_are_queued_for_it() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(88, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(88),
            StoryEventKind::TagAdded {
                label: "current-iteration".to_string(),
            },
        ))
        .unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(88),
            StoryEventKind::TagAdded {
                label: "needs-attention".to_string(),
            },
        ))
        .unwrap();
    assert!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(88)))
            .is_none(),
        "the create_iteration mirror leaves the backing story absent while its tag writes queue"
    );

    orchestrator.refresh_story(88).await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(88)))
            .unwrap()
            .state,
        Work::NotStarted,
        "an absent story carries no optimistic state to protect, so refresh hydrates it even with queued tag writes"
    );
}

#[tokio::test]
async fn refresh_story_seeds_when_no_local_write_is_queued() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(77, Work::Acceptance)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    assert!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(77)))
            .is_none(),
        "store starts empty"
    );
    orchestrator.refresh_story(77).await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(77)))
            .unwrap()
            .state,
        Work::Acceptance,
        "with an empty outbox refresh hydrates the story exactly as before WR-1"
    );
}

#[tokio::test]
async fn a_stale_ingest_after_the_accept_write_drains_does_not_revert_the_accepted_state() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(45, Work::Acceptance)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator.ingest_once().await.unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(45),
            StoryEventKind::StateTransitioned {
                new_state: Work::Accepted,
                placement: None,
            },
        ))
        .unwrap();
    orchestrator.drain_pass().await.unwrap();

    adapter.seed_stories(vec![seed_story(45, Work::Acceptance)]);
    orchestrator.ingest_once().await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(45)))
            .unwrap()
            .state,
        Work::Accepted,
        "the post-drain window: an undated poll snapshot read before the accept push is reflected remotely must NOT revert the optimistic Accepted state"
    );
}

#[tokio::test]
async fn ingest_reconciles_fully_once_the_queue_for_the_story_has_drained() {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(44, Work::NotStarted)]);
    let orchestrator = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let orch_dyn: Arc<dyn RefreshDriver> = orchestrator.clone();
    let handle = SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orch_dyn);
    let select = handle.select();

    orchestrator.ingest_once().await.unwrap();
    orchestrator
        .mutate(StoryEvent::new(
            meta(),
            StoryId::new(44),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: None,
            },
        ))
        .unwrap();
    orchestrator.drain_pass().await.unwrap();

    let mut reconciled = seed_story(44, Work::InProgress);
    reconciled.title = "remote-renamed".to_string();
    adapter.seed_stories(vec![reconciled]);

    orchestrator.ingest_once().await.unwrap();

    assert_eq!(
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(44)))
            .unwrap()
            .title,
        "remote-renamed",
        "the first ingest after the queue empties reconciles with full remote truth"
    );
}
