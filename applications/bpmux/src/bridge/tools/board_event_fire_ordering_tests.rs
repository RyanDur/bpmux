#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::{BoardStore, StoreChange};
use crate::product_management::story::{BoardState, Stage, Story, StoryId};
use chrono::Utc;
use std::sync::Arc;

fn meta() -> EventMeta {
    EventMeta {
        actor: "test".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}

fn seed_story_at(store: &BoardStore, number: u64, state: BoardState) {
    let story = Story::new_minimal(
        StoryId::new(number),
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
    expected_new_state: BoardState,
) {
    let mut rx = store.subscribe();
    store.apply(&StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(number),
        new_state: expected_new_state,
    });
    let event = rx.try_recv().expect(
        "StoryEvent::StateTransitioned delivered to subscriber after apply (#1282 invariant)",
    );
    match event {
        StoreChange::Event(StoryEvent::StateTransitioned {
            new_state, id: n, ..
        }) => {
            assert_eq!(n, StoryId::new(number));
            assert_eq!(new_state, expected_new_state);
        }
        other => panic!("expected StateTransitioned, got {other:?}"),
    }
    assert_eq!(
        store.story(StoryId::new(number)).unwrap().state,
        expected_new_state,
        "store reflects the new state after apply returns"
    );
}

#[tokio::test]
async fn start_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 100, BoardState::Ready);
    assert_state_transition_fires(&store, 100, BoardState::InProgress(Stage::Dev));
}

#[tokio::test]
async fn deliver_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 200, BoardState::InProgress(Stage::Dev));
    assert_state_transition_fires(&store, 200, BoardState::Delivered);
}

#[tokio::test]
async fn accept_story_state_transition_dispatches_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 300, BoardState::Delivered);
    assert_state_transition_fires(&store, 300, BoardState::Accepted);
}

#[tokio::test]
async fn sister_to_1274_state_transition_reaches_cascade_listener() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 400, BoardState::Delivered);
    let mut rx = store.subscribe();

    store.apply(&StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(400),
        new_state: BoardState::Accepted,
    });

    let event = rx
        .try_recv()
        .expect("cascade-listener receives StateTransitioned event after apply (#1274 sister)");
    assert!(
        matches!(
            event,
            StoreChange::Event(StoryEvent::StateTransitioned { .. })
        ),
        "event delivered to subscriber is the StateTransitioned we sent"
    );
    assert_eq!(
        store.story(StoryId::new(400)).unwrap().state,
        BoardState::Accepted,
        "store reflects the transition after apply completes"
    );
}

#[tokio::test]
async fn multiple_subscribers_each_receive_state_transition_event() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 500, BoardState::Ready);
    let mut rx_a = store.subscribe();
    let mut rx_b = store.subscribe();

    store.apply(&StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(500),
        new_state: BoardState::InProgress(Stage::Dev),
    });

    let event_a = rx_a.try_recv().expect("subscriber A receives event");
    let event_b = rx_b.try_recv().expect("subscriber B receives event");
    assert!(matches!(
        event_a,
        StoreChange::Event(StoryEvent::StateTransitioned { .. })
    ));
    assert!(matches!(
        event_b,
        StoreChange::Event(StoryEvent::StateTransitioned { .. })
    ));
}

#[tokio::test]
async fn broadcast_fires_after_reduce_so_listeners_observe_new_state() {
    let store = Arc::new(BoardStore::new());
    seed_story_at(&store, 600, BoardState::Ready);

    let observed_at_hook: Arc<std::sync::Mutex<Option<BoardState>>> =
        Arc::new(std::sync::Mutex::new(None));
    let observed_clone = Arc::clone(&observed_at_hook);
    let store_for_hook = Arc::clone(&store);
    store.set_test_post_broadcast_hook(move || {
        let state = store_for_hook.story(StoryId::new(600)).map(|s| s.state);
        *observed_clone.lock().unwrap_or_else(|e| e.into_inner()) = state;
    });

    store.apply(&StoryEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(600),
        new_state: BoardState::InProgress(Stage::Dev),
    });

    let observed = observed_at_hook
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .expect("hook fired immediately after event_tx.send broadcast the event");
    assert_eq!(
        observed,
        BoardState::InProgress(Stage::Dev),
        "the reducer mutated state BEFORE event_tx.send broadcast to listeners, \
         so a listener observes the reduced state per the redux pipeline \
         (dispatch -> middleware -> reducer -> listeners): #1282 listener-after-reducer invariant"
    );
    assert_eq!(
        store.story(StoryId::new(600)).unwrap().state,
        BoardState::InProgress(Stage::Dev),
        "store reflects new state after apply returns"
    );
}
