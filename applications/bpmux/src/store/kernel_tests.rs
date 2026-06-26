#![allow(clippy::unwrap_used)]

use super::kernel::{Listener, ListenerFuture, Reaction, Store};
use super::test_support::eventually;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Counter {
    value: i32,
}

#[derive(Clone, Debug)]
enum Count {
    Increment,
    Add(i32),
}

fn reduce(state: &Counter, action: &Count) -> Counter {
    match action {
        Count::Increment => Counter {
            value: state.value + 1,
        },
        Count::Add(amount) => Counter {
            value: state.value + amount,
        },
    }
}

fn counter_store() -> Arc<Store<Counter, Count>> {
    Store::new(Counter::default(), Box::new(reduce))
}

fn value_of(store: &Store<Counter, Count>) -> i32 {
    store.select(|state| state.value)
}

#[tokio::test]
async fn the_reducer_produces_new_state_for_each_action() {
    let store = counter_store();
    store.dispatch(Count::Increment);
    store.dispatch(Count::Add(5));
    assert_eq!(value_of(&store), 6);
}

struct RecordTransitions {
    log: Arc<Mutex<Vec<(i32, i32)>>>,
}

impl Listener<Counter, Count> for RecordTransitions {
    fn on_action(&self, reaction: Reaction<Counter, Count>) -> ListenerFuture {
        let log = Arc::clone(&self.log);
        Box::pin(async move {
            log.lock()
                .unwrap()
                .push((reaction.previous.value, reaction.current.value));
        })
    }
}

#[tokio::test]
async fn a_listener_sees_the_action_and_the_previous_and_current_state() {
    let store = counter_store();
    let log = Arc::new(Mutex::new(Vec::new()));
    store.add_listener(Arc::new(RecordTransitions {
        log: Arc::clone(&log),
    }));

    store.dispatch(Count::Increment);
    eventually("first transition recorded", || {
        log.lock().unwrap().len() == 1
    })
    .await;
    store.dispatch(Count::Add(5));
    eventually("second transition recorded", || {
        log.lock().unwrap().len() == 2
    })
    .await;

    assert_eq!(*log.lock().unwrap(), vec![(0, 1), (1, 6)]);
}

struct ReinjectIncrementOnAdd;

impl Listener<Counter, Count> for ReinjectIncrementOnAdd {
    fn on_action(&self, reaction: Reaction<Counter, Count>) -> ListenerFuture {
        Box::pin(async move {
            if matches!(reaction.action, Count::Add(_)) {
                (reaction.dispatch)(Count::Increment);
            }
        })
    }
}

#[tokio::test]
async fn a_listener_can_dispatch_a_follow_up_action() {
    let store = counter_store();
    store.add_listener(Arc::new(ReinjectIncrementOnAdd));

    store.dispatch(Count::Add(10));
    eventually("listener reinjected the follow-up increment", || {
        value_of(&store) == 11
    })
    .await;

    assert_eq!(value_of(&store), 11);
}
