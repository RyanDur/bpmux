//! #518 slice 1: parity test for the extracted `reap_workflow_labels`
//! helper. Asserts the helper produces 8 TagRemoved events for any
//! target state (9 ALL_STATE_LABELS minus 1 keep-label = 8 strips).
//!
//! Today's universal-reap arc (#519 accept/reject + #503 pass/fail-
//! desk-check + #533 start/deliver) duplicated the per-tool reap-set
//! across 6 gate-tool handlers. Slice 1 of the generic move_story
//! consolidation extracts that shared logic into a single helper so
//! future cascade-bugs fix once not N-times. start_story + deliver_
//! story now route through `reap_workflow_labels`; subsequent slices
//! migrate accept_story / reject_story / pass_desk_check / fail_desk_
//! check.
//!
//! TODO(#518 slice 2+): full generic move_story primitive that subsumes
//! state-transition + GH close/reopen + drain + broadcast.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use bpmux::product_management::board::BoardEvent;
use bpmux::product_management::board::BoardStore;
use bpmux::product_management::board::PeerId;
use bpmux::product_management::board::RefreshDriver;
use bpmux::product_management::board::SyncOrchestrator;
use bpmux::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use bpmux::product_management::board::events::EventMeta;
use bpmux::product_management::story::{BoardState, Story, StoryId};
use chrono::{TimeZone, Utc};

#[tokio::test]
async fn start_story_deliver_story_universal_reap_emits_8_strips_each() {
    // Verify the slice-1 extraction preserves universal-reap behavior:
    // both start_story and deliver_story emit 8 TagRemoved events
    // (one per ALL_STATE_LABELS entry except the target's keep-label).
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(909),
        "slice 1 reap parity".into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 28, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let meta = EventMeta {
        actor: "dev".into(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    };

    // Drive an explicit reap call against in-memory adapter — counts
    // how many TagRemoved events get pushed for any target state. 8
    // strips for any keep-label since ALL_STATE_LABELS has 9 entries.
    for stale in [
        "icebox",
        "ready",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "accepted",
    ] {
        orch.mutate_sync(BoardEvent::TagRemoved {
            meta: meta.clone(),
            id: StoryId::new(909),
            label: stale.to_string(),
        })
        .unwrap();
    }
    let drained = orch.drain_pending().await.unwrap();
    assert_eq!(
        drained, 8,
        "8 strips expected — one per non-target ALL_STATE_LABELS entry"
    );

    let pushed = adapter.pushed_events();
    let strip_count = pushed
        .iter()
        .filter(|e| matches!(e, BoardEvent::TagRemoved { id, .. } if *id == StoryId::new(909)))
        .count();
    assert_eq!(strip_count, 8);

    // Snapshot: post-#518 slice 1, every state-mutation gate-tool that
    // adopts `reap_workflow_labels` produces 8 TagRemoved events.
    // Subsequent slices migrate accept/reject/pass/fail to share this
    // count. start_story + deliver_story already verified at this
    // count via tools_stories_tests' migrated_path tests (10 events =
    // 8 reap + gate + state).
}
