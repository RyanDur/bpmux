#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use crate::integrations::github::gh_label_codec::GhLabelCodec;
use product_management::story::StoryId;
use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use source_sync::{MutationId, PendingWrite, QueuedWrite};

use crate::store::sync::{SyncAction, SyncState, reduce};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 7, 8, 0, 0, 0).unwrap(),
    }
}

fn tag_added(story: u64, label: &str) -> PendingWrite {
    PendingWrite::from_event(
        &GhLabelCodec,
        StoryEvent::new(
            meta(),
            StoryId::new(story),
            StoryEventKind::TagAdded {
                label: label.to_string(),
            },
        ),
    )
}

fn tag_removed(story: u64, label: &str) -> PendingWrite {
    PendingWrite::from_event(
        &GhLabelCodec,
        StoryEvent::new(
            meta(),
            StoryId::new(story),
            StoryEventKind::TagRemoved {
                label: label.to_string(),
            },
        ),
    )
}

fn hydrated_write(story: u64, label: &str, id: u64) -> QueuedWrite {
    QueuedWrite::minted(MutationId(id), tag_added(story, label))
}

fn queued(state: &SyncState, action: SyncAction) -> SyncState {
    reduce(state, &action).expect("action changes the queue")
}

fn story_order(state: &SyncState) -> Vec<u64> {
    state
        .queue
        .iter()
        .map(|w| w.event.story_id().value())
        .collect()
}

fn mutation_ids(state: &SyncState) -> Vec<u64> {
    state.queue.iter().map(|w| w.id.0).collect()
}

fn labels(state: &SyncState) -> Vec<(&'static str, String)> {
    state
        .queue
        .iter()
        .filter_map(|w| match &w.event.kind {
            StoryEventKind::TagAdded { label } => Some(("A", label.clone())),
            StoryEventKind::TagRemoved { label } => Some(("R", label.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn write_queued_appends_in_fifo_order() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b"),
        },
    );
    assert_eq!(
        story_order(&state),
        vec![1, 2],
        "writes append in dispatch order"
    );
}

#[test]
fn writes_queued_appends_the_whole_batch_in_order() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WritesQueued(vec![tag_added(1, "a"), tag_added(2, "b")]),
    );
    assert_eq!(story_order(&state), vec![1, 2]);
}

#[test]
fn two_write_queued_dispatches_mint_distinct_increasing_ids() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b"),
        },
    );
    assert_eq!(
        mutation_ids(&state),
        vec![1, 2],
        "the reducer mints a distinct, increasing id per queued write"
    );
}

#[test]
fn writes_queued_mints_sequential_ids_in_vec_order() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WritesQueued(vec![
            tag_added(1, "a"),
            tag_added(2, "b"),
            tag_added(3, "c"),
        ]),
    );
    assert_eq!(
        mutation_ids(&state),
        vec![1, 2, 3],
        "a batch mints sequential ids matching vec order"
    );
}

#[test]
fn hydrated_max_id_seeds_next_mint_so_a_fresh_write_never_collides() {
    let hydrated =
        crate::store::app::AppState::default().with_outbox(vec![hydrated_write(9, "ready", 7)]);
    let state = (*hydrated.sync).clone();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    let fresh_id = state
        .queue
        .iter()
        .find(|w| w.event.story_id().value() == 1)
        .expect("the fresh write is queued")
        .id
        .0;
    assert_eq!(
        fresh_id, 8,
        "the fresh mint is max-persisted-id + 1 — it never collides with a hydrated row's id"
    );
}

#[test]
fn an_absorbed_iteration_label_write_still_advances_the_counter() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.100"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.200"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "ready"),
        },
    );
    let third_id = state
        .queue
        .iter()
        .find(|w| w.event.story_id().value() == 2)
        .expect("the third write is queued")
        .id
        .0;
    assert_eq!(
        third_id, 3,
        "the coalesced second write still consumed its id — the counter advanced past the absorbed write, so the next id is 3 not 2"
    );
}

#[test]
fn coalesce_add_then_add_keeps_the_last_add() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.100"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.200"),
        },
    );
    assert_eq!(
        labels(&state),
        vec![("A", "iteration-Pegasus.200".to_string())],
        "same-class adds coalesce to the last add"
    );
}

#[test]
fn coalesce_remove_then_add_keeps_first_remove_and_last_add() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_removed(1, "iteration-Pegasus.300"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.50"),
        },
    );
    assert_eq!(
        labels(&state),
        vec![
            ("R", "iteration-Pegasus.300".to_string()),
            ("A", "iteration-Pegasus.50".to_string()),
        ],
        "a same-class remove+add coalesces to first-remove + last-add"
    );
}

#[test]
fn non_iteration_labels_do_not_coalesce() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "ready"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "blocked"),
        },
    );
    assert_eq!(
        state.queue.len(),
        2,
        "labels with no iteration-class stay distinct"
    );
}

#[test]
fn write_pushed_removes_exactly_its_id() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b"),
        },
    );
    let state = queued(&state, SyncAction::WritePushed { id: MutationId(1) });
    assert_eq!(
        mutation_ids(&state),
        vec![2],
        "only the pushed id is removed"
    );
}

#[test]
fn write_dropped_removes_exactly_its_id() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b"),
        },
    );
    let state = queued(&state, SyncAction::WriteDropped { id: MutationId(2) });
    assert_eq!(
        mutation_ids(&state),
        vec![1],
        "only the dropped id is removed"
    );
}

#[test]
fn removing_an_absent_id_is_a_no_op() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a"),
        },
    );
    assert!(
        reduce(&state, &SyncAction::WritePushed { id: MutationId(99) }).is_none(),
        "removing an id not in the queue reports no change"
    );
}

#[test]
fn start_ingest_polling_command_is_pure_intent_not_actuality() {
    let state = SyncState::default();
    assert!(
        reduce(&state, &SyncAction::StartIngestPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}

#[test]
fn stop_ingest_polling_command_is_pure_intent_not_actuality() {
    let state = SyncState::default();
    assert!(
        reduce(&state, &SyncAction::StopIngestPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}

#[test]
fn start_hydration_polling_command_is_pure_intent_not_actuality() {
    let state = SyncState::default();
    assert!(
        reduce(&state, &SyncAction::StartHydrationPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}

#[test]
fn stop_hydration_polling_command_is_pure_intent_not_actuality() {
    let state = SyncState::default();
    assert!(
        reduce(&state, &SyncAction::StopHydrationPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}
