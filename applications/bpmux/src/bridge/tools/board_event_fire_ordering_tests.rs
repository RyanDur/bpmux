#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

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
    // #1274 sister-regression: the original failure mode was a
    // state-mutation path that committed without firing the event,
    // leaving close_iteration's promote-batch scan with no trigger.
    // BoardStore::apply guarantees every state-mutation through the
    // canonical chokepoint delivers an event to subscribers.
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
    assert!(
        matches!(event, BoardEvent::StateTransitioned { .. }),
        "event delivered to subscriber is the StateTransitioned we sent"
    );
    assert_eq!(
        store.story(400).unwrap().state,
        StoryState::Accepted,
        "store reflects the transition after apply completes"
    );
}

#[tokio::test]
async fn multiple_subscribers_each_receive_state_transition_event() {
    // Reliability dimension: broadcast delivers the event to ALL
    // subscribers, not just one. Cascade-effects in different modules
    // (close_iteration in one path, icebox_unified cascade-clean in
    // another) each subscribe independently.
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
    assert!(matches!(event_a, BoardEvent::StateTransitioned { .. }));
    assert!(matches!(event_b, BoardEvent::StateTransitioned { .. }));
}

#[tokio::test]
async fn pre_commit_fire_ordering_observable_via_post_send_hook() {
    // Deterministic ordering test (#1282): installs a #[cfg(test)] hook
    // on BoardStore that fires synchronously between event_tx.send and
    // the write-lock acquire. The hook reads the store-state for the
    // story being mutated — at this point the event has been broadcast
    // but the mutation has NOT yet committed, so the hook observes the
    // OLD state. This pins the pre-commit fire-ordering observably,
    // discriminating between the invariant (OLD visible) and a
    // post-commit alternative (NEW visible).
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 600, StoryState::Ready);

    let observed_at_hook: Arc<std::sync::Mutex<Option<StoryState>>> =
        Arc::new(std::sync::Mutex::new(None));
    let observed_clone = Arc::clone(&observed_at_hook);
    let store_for_hook = Arc::clone(&store);
    store.set_test_post_send_hook(move || {
        let state = store_for_hook.story(600).map(|s| s.state);
        *observed_clone.lock().unwrap_or_else(|e| e.into_inner()) = state;
    });

    store.apply(&BoardEvent::StateTransitioned {
        meta: meta(),
        number: 600,
        new_state: StoryState::InProgress,
    });

    let observed = observed_at_hook
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .expect("hook fired between event_tx.send and write-lock acquire");
    assert_eq!(
        observed,
        StoryState::Ready,
        "hook ran AFTER event_tx.send but BEFORE write-lock acquired the mutation; \
         observed pre-mutation state per #1282 fire-ordering invariant"
    );
    assert_eq!(
        store.story(600).unwrap().state,
        StoryState::InProgress,
        "store reflects new state after apply returns"
    );
}
