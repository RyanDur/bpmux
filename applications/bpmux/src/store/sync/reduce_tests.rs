#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::story::StoryId;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::sync::{MutationId, QueuedWrite};

use super::{SyncAction, SyncState, reduce};

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 7, 8, 0, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn tag_added(story: u64, label: &str, id: u64) -> QueuedWrite {
    QueuedWrite::from_event(
        &GhLabelCodec,
        MutationId(id),
        StoryEvent::new(
            meta(),
            StoryId::new(story),
            StoryEventKind::TagAdded {
                label: label.to_string(),
            },
        ),
    )
}

fn tag_removed(story: u64, label: &str, id: u64) -> QueuedWrite {
    QueuedWrite::from_event(
        &GhLabelCodec,
        MutationId(id),
        StoryEvent::new(
            meta(),
            StoryId::new(story),
            StoryEventKind::TagRemoved {
                label: label.to_string(),
            },
        ),
    )
}

fn queued(state: &SyncState, action: SyncAction) -> SyncState {
    reduce(state, &action).expect("action changes the queue")
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
            write: tag_added(1, "a", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b", 2),
        },
    );
    let ids: Vec<u64> = state.queue.iter().map(|w| w.id.0).collect();
    assert_eq!(ids, vec![1, 2], "writes append in dispatch order");
}

#[test]
fn writes_queued_appends_the_whole_batch_in_order() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WritesQueued(vec![tag_added(1, "a", 1), tag_added(2, "b", 2)]),
    );
    let ids: Vec<u64> = state.queue.iter().map(|w| w.id.0).collect();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn coalesce_add_then_add_keeps_the_last_add() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.100", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.200", 2),
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
            write: tag_removed(1, "iteration-Pegasus.300", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "iteration-Pegasus.50", 2),
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
            write: tag_added(1, "ready", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "blocked", 2),
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
            write: tag_added(1, "a", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b", 2),
        },
    );
    let state = queued(&state, SyncAction::WritePushed { id: MutationId(1) });
    let ids: Vec<u64> = state.queue.iter().map(|w| w.id.0).collect();
    assert_eq!(ids, vec![2], "only the pushed id is removed");
}

#[test]
fn write_dropped_removes_exactly_its_id() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a", 1),
        },
    );
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(2, "b", 2),
        },
    );
    let state = queued(&state, SyncAction::WriteDropped { id: MutationId(2) });
    let ids: Vec<u64> = state.queue.iter().map(|w| w.id.0).collect();
    assert_eq!(ids, vec![1], "only the dropped id is removed");
}

#[test]
fn removing_an_absent_id_is_a_no_op() {
    let state = SyncState::default();
    let state = queued(
        &state,
        SyncAction::WriteQueued {
            write: tag_added(1, "a", 1),
        },
    );
    assert!(
        reduce(&state, &SyncAction::WritePushed { id: MutationId(99) }).is_none(),
        "removing an id not in the queue reports no change"
    );
}
