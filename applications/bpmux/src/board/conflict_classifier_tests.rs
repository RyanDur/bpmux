//! #1195 slice-A substrate tests: QueuedEvent wrapper + pre_state
//! capture in mutate() + outbox schema backward-compat + ConflictKind
//! type surface + SyncStatus.conflicts_pending field. The classifier
//! itself returns None for slice A (substrate-only ship); slice B+
//! plugs in the real divergence detection.
//!
//! These tests pin the SUBSTRATE shape so slice B can rely on it:
//! - QueuedEvent serializable + back-compat with legacy bare-event
//!   outbox files
//! - mutate() captures pre_state BEFORE applying the event
//! - SyncStatus carries the conflicts_pending field

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::orchestrator::{
    ConflictKind, QueuedEvent, SyncOrchestrator, classify_conflict, load_outbox_from_disk,
};
use crate::board::peer::PeerId;
use crate::board::store::BoardStore;

fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 5, 22, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn seed_story(number: u64, state: StoryState) -> Story {
    Story::new_minimal(
        number,
        format!("title-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[tokio::test]
async fn mutate_captures_pre_state_snapshot_before_local_apply() {
    // Substrate-pin: pre_state in QueuedEvent reflects what the
    // store looked like BEFORE the event applied. Without this,
    // slice B's divergence-detection has nothing to compare
    // server-truth against (per
    // `feedback_mutate_sync_breaks_drain_time_idempotent_check`).
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(500, StoryState::Ready)]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch =
        SyncOrchestrator::with_outbox_path(Arc::clone(&store), Arc::clone(&adapter), path.clone())
            .unwrap();
    orch.refresh_all().await.unwrap();

    orch.mutate(BoardEvent::StateTransitioned {
        meta: meta(),
        number: 500,
        new_state: StoryState::InProgress,
    })
    .unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    let pre = entry
        .pre_state
        .as_ref()
        .expect("pre_state captured at queue-time");
    assert_eq!(
        pre.state,
        StoryState::Ready,
        "pre_state must reflect the pre-mutation state, not post"
    );
    assert!(matches!(
        entry.event,
        BoardEvent::StateTransitioned {
            new_state: StoryState::InProgress,
            ..
        }
    ));
}

#[tokio::test]
async fn mutate_pre_state_none_when_story_not_in_store_at_queue_time() {
    // Pre-ingest case: agent fires a mutation against a story the
    // store hasn't ingested yet. pre_state is None; classifier
    // degrades to no-conflict-detection (push normally) per the
    // spec's backward-compat semantics.
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outbox.json");
    let orch =
        SyncOrchestrator::with_outbox_path(Arc::clone(&store), Arc::clone(&adapter), path.clone())
            .unwrap();

    orch.mutate(BoardEvent::TagAdded {
        meta: meta(),
        number: 999,
        label: "blocked".to_string(),
    })
    .unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(
        entries.front().unwrap().pre_state.is_none(),
        "pre_state must be None when canonical store doesn't have the story at queue-time"
    );
}

#[tokio::test]
async fn outbox_schema_back_compat_reads_legacy_bare_boardevent_file() {
    // Backward-compat: a board-outbox.json written by pre-#1195
    // bpmux contains `Vec<BoardEvent>` (bare events). load_outbox
    // _from_disk must accept that shape + map each entry to a
    // QueuedEvent with pre_state: None so slice-B classifier
    // degrades safely (no-conflict-detection on legacy entries).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-outbox.json");
    let legacy_event = BoardEvent::TagAdded {
        meta: meta(),
        number: 42,
        label: "ready".to_string(),
    };
    let legacy_payload = serde_json::to_vec(&vec![legacy_event]).unwrap();
    std::fs::write(&path, legacy_payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    assert!(matches!(
        entry.event,
        BoardEvent::TagAdded { number: 42, .. }
    ));
    assert!(
        entry.pre_state.is_none(),
        "legacy entries map to QueuedEvent with pre_state: None"
    );
}

#[tokio::test]
async fn outbox_round_trips_new_shape_through_disk() {
    // Forward-compat: a QueuedEvent persisted to disk reads back
    // identical (event + pre_state). Pin so future schema changes
    // can't silently drop the pre_state field.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new-outbox.json");
    let pre = seed_story(7, StoryState::Delivered);
    let queued = QueuedEvent::new(
        BoardEvent::StateTransitioned {
            meta: meta(),
            number: 7,
            new_state: StoryState::Accepted,
        },
        Some(pre.clone()),
    );
    let payload = serde_json::to_vec(&vec![queued]).unwrap();
    std::fs::write(&path, payload).unwrap();

    let entries = load_outbox_from_disk(&path).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = entries.front().unwrap();
    let restored_pre = entry.pre_state.as_ref().unwrap();
    assert_eq!(restored_pre.number, pre.number);
    assert_eq!(restored_pre.state, StoryState::Delivered);
}

#[test]
fn classify_conflict_returns_none_for_slice_a_pending_real_classifier() {
    // Slice A surfaces the type + signature; slice B replaces the
    // body with real divergence-detection semantics. Pin "always
    // None for now" so slice-B's intentional behavior change
    // surfaces here as a test-update rather than silently shipping
    // false positives.
    let pre = seed_story(1, StoryState::Ready);
    let cur = seed_story(1, StoryState::InProgress);
    let event = BoardEvent::StateTransitioned {
        meta: meta(),
        number: 1,
        new_state: StoryState::InProgress,
    };
    assert_eq!(classify_conflict(Some(&pre), &event, Some(&cur)), None);
}

#[test]
fn conflict_kind_serde_round_trip() {
    // Pin: ConflictKind enum is serializable so SyncStatus +
    // future RPC surfaces can carry it. Slice B will surface this
    // in get_sync_status output; today's test catches if a
    // refactor accidentally drops Serialize/Deserialize.
    let kind = ConflictKind::IdempotentSkip;
    let json = serde_json::to_string(&kind).unwrap();
    let back: ConflictKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, kind);
}

#[tokio::test]
async fn drain_resets_conflicts_pending_to_zero_in_slice_a() {
    // Slice A: classifier returns None for everything → drain
    // never marks any entry conflict-pending → conflicts_pending
    // stays 0. Slice B+ wires in real classification + this test
    // updates accordingly.
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![seed_story(800, StoryState::Ready)]);
    let orch: Arc<dyn crate::board::RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    orch.mutate_sync(BoardEvent::StateTransitioned {
        meta: meta(),
        number: 800,
        new_state: StoryState::InProgress,
    })
    .unwrap();

    let _ = orch.drain_pending().await.unwrap();
    assert_eq!(orch.snapshot_status().conflicts_pending, 0);
}
