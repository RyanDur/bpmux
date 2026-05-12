#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1282: BoardEvent::StateTransitioned fire-ordering invariant
// at BoardStore::apply. The invariant: event_tx.send fires BEFORE the
// store-mutation acquires the write-lock + commits — code-level
// guarantee documented at BoardStore::apply. Today's #1274 surfaced
// the underlying class: a state-mutation path that committed without
// the corresponding event firing, leaving downstream listeners
// (close_iteration promote-batch scan, icebox_unified cascade-clean)
// unable to detect the transition.
//
// What this file actually verifies (practically observable):
// 1. Every state-transition via BoardStore::apply delivers a
//    BoardEvent::StateTransitioned to subscribers — the basic
//    "event fires reliably" guarantee.
// 2. The store reflects the new state after apply returns — atomic
//    commit semantic.
// 3. The #1274 regression scenario: close_iteration's downstream
//    cascade-trigger path receives the event so promote-batch can
//    react.
//
// What the invariant is NOT (and why we don't pin it via subscribers):
// the async subscriber pattern doesn't deterministically observe the
// pre-mutation store-state because the subscriber-task isn't
// guaranteed to run during apply. The fire-ordering is a code-level
// property documented at the apply chokepoint; runtime tests verify
// the consequence (event always delivered) rather than the
// implementation (event-send before write-lock).

use crate::board::BoardStore;
use crate::board::domain::{Story, StoryState};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::peer::PeerId;
use chrono::Utc;
use std::sync::Arc;

fn meta() -> EventMeta {
    EventMeta {
        actor: "test".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}

fn seed_story_at(store: &BoardStore, number: u64, state: StoryState) {
    let story = Story::new_minimal(
        number,
        format!("[Feature] story {number}"),
        state,
        Utc::now(),
        "pm".to_string(),
    );
    store.seed_all(vec![story]);
}

fn assert_state_transition_fires(
    store: &Arc<BoardStore>,
    number: u64,
    expected_new_state: StoryState,
) {
    let mut rx = store.subscribe();
    store.apply(&BoardEvent::StateTransitioned {
        meta: meta(),
        number,
        new_state: expected_new_state,
    });
    let event = rx.try_recv().expect(
        "BoardEvent::StateTransitioned delivered to subscriber after apply (#1282 invariant)",
    );
    match event {
        BoardEvent::StateTransitioned {
            new_state,
            number: n,
            ..
        } => {
            assert_eq!(n, number);
            assert_eq!(new_state, expected_new_state);
        }
        other => panic!("expected StateTransitioned, got {other:?}"),
    }
    assert_eq!(
        store.story(number).unwrap().state,
        expected_new_state,
        "store reflects the new state after apply returns"
    );
}

#[tokio::test]
async fn start_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 100, StoryState::Ready);
    assert_state_transition_fires(&store, 100, StoryState::InProgress);
}

#[tokio::test]
async fn deliver_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 200, StoryState::InProgress);
    assert_state_transition_fires(&store, 200, StoryState::Delivered);
}

#[tokio::test]
async fn accept_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 300, StoryState::Delivered);
    assert_state_transition_fires(&store, 300, StoryState::Accepted);
}

#[tokio::test]
async fn sister_to_1274_state_transition_reaches_cascade_listener() {
    // #1274 sister-regression: the original failure mode was that a
    // state-mutation path completed without firing the event, leaving
    // close_iteration's promote-batch scan unable to detect the
    // transition. The invariant at BoardStore::apply guarantees the
    // event is delivered for every state-mutation that goes through
    // the canonical chokepoint — so cascade-listeners (close_iteration,
    // cascade-clean, etc.) always receive their trigger.
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 400, StoryState::Delivered);
    let mut rx = store.subscribe();

    store.apply(&BoardEvent::StateTransitioned {
        meta: meta(),
        number: 400,
        new_state: StoryState::Accepted,
    });

    let event = rx
        .try_recv()
        .expect("cascade-listener receives StateTransitioned event after apply (#1274 sister)");
    matches!(event, BoardEvent::StateTransitioned { .. });
    assert_eq!(
        store.story(400).unwrap().state,
        StoryState::Accepted,
        "store reflects the transition after apply completes"
    );
}

#[tokio::test]
async fn multiple_subscribers_each_receive_state_transition_event() {
    // Reliability dimension: the broadcast channel delivers the event
    // to ALL subscribers, not just one. Cascade-effects in different
    // modules (close_iteration in one path, icebox_unified cascade-
    // clean in another) each subscribe independently.
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 500, StoryState::Ready);
    let mut rx_a = store.subscribe();
    let mut rx_b = store.subscribe();

    store.apply(&BoardEvent::StateTransitioned {
        meta: meta(),
        number: 500,
        new_state: StoryState::InProgress,
    });

    let event_a = rx_a.try_recv().expect("subscriber A receives event");
    let event_b = rx_b.try_recv().expect("subscriber B receives event");
    matches!(event_a, BoardEvent::StateTransitioned { .. });
    matches!(event_b, BoardEvent::StateTransitioned { .. });
}
