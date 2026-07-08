#![allow(clippy::unwrap_used, clippy::panic)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::kernel::{DeliveryMode, Listener, ListenerFuture, Middleware, Reaction, Store};

async fn eventually(label: &str, mut condition: impl FnMut() -> bool) {
    for _ in 0..400 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("eventually({label}): condition not satisfied within 2s");
}

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

struct DoubleAddWhileBelowTen;

impl Middleware<Counter, Count> for DoubleAddWhileBelowTen {
    fn transform(&self, state: &Counter, action: Count) -> Count {
        match action {
            Count::Add(amount) if state.value < 10 => Count::Add(amount * 2),
            other => other,
        }
    }
}

#[tokio::test]
async fn middleware_transforms_the_action_against_current_state_before_the_reducer() {
    let store = counter_store();
    store.add_middleware(Arc::new(DoubleAddWhileBelowTen));

    store.dispatch(Count::Add(3));
    assert_eq!(value_of(&store), 6);

    store.dispatch(Count::Add(3));
    assert_eq!(value_of(&store), 12);

    store.dispatch(Count::Add(3));
    assert_eq!(value_of(&store), 15);
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

struct RecordDeliveries {
    mode: DeliveryMode,
    log: Arc<Mutex<Vec<(i32, i32)>>>,
}

impl Listener<Counter, Count> for RecordDeliveries {
    fn on_action(&self, reaction: Reaction<Counter, Count>) -> ListenerFuture {
        let log = Arc::clone(&self.log);
        let transition = (reaction.previous.value, reaction.current.value);
        Box::pin(async move {
            log.lock().unwrap().push(transition);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        self.mode
    }
}

#[tokio::test]
async fn latest_state_coalesces_a_synchronous_burst_and_feeds_the_high_water_mark_previous() {
    let store = counter_store();
    let log = Arc::new(Mutex::new(Vec::new()));
    store.add_listener(Arc::new(RecordDeliveries {
        mode: DeliveryMode::LatestState,
        log: Arc::clone(&log),
    }));

    store.dispatch(Count::Increment);
    store.dispatch(Count::Increment);
    store.dispatch(Count::Increment);

    eventually("the coalesced burst is delivered once", || {
        log.lock().unwrap().len() == 1
    })
    .await;
    tokio::task::yield_now().await;

    assert_eq!(
        *log.lock().unwrap(),
        vec![(0, 3)],
        "the two intermediate states are discarded; the single delivery carries current=3 and previous=0 — the worker's high-water mark, NOT the immediate predecessor 2 — so any slice that changed across the whole burst is still detectable"
    );
}

#[tokio::test]
async fn ordered_delivers_every_reaction_in_dispatch_order_without_coalescing() {
    let store = counter_store();
    let log = Arc::new(Mutex::new(Vec::new()));
    store.add_listener(Arc::new(RecordDeliveries {
        mode: DeliveryMode::Ordered,
        log: Arc::clone(&log),
    }));

    store.dispatch(Count::Increment);
    store.dispatch(Count::Increment);
    store.dispatch(Count::Increment);

    eventually("every reaction in the burst is delivered", || {
        log.lock().unwrap().len() == 3
    })
    .await;

    assert_eq!(
        *log.lock().unwrap(),
        vec![(0, 1), (1, 2), (2, 3)],
        "the SAME burst under ORDERED yields one delivery per reaction, in dispatch order — proving ORDERED never coalesces"
    );
}
