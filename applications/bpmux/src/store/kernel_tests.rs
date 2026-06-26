#![allow(clippy::unwrap_used)]

use super::kernel::{Changed, Middleware, Store, StoreApi};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Counter {
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Count {
    Increment,
    Add(i32),
    Ignored,
}

fn reduce(state: &mut Counter, action: &Count) -> Changed {
    match action {
        Count::Increment => {
            state.value += 1;
            Changed::Yes
        }
        Count::Add(amount) => {
            state.value += amount;
            Changed::Yes
        }
        Count::Ignored => Changed::No,
    }
}

fn store_with(middleware: Vec<Arc<dyn Middleware<Counter, Count>>>) -> Store<Counter, Count> {
    Store::new(Counter::default(), Box::new(reduce), middleware)
}

fn value_of(store: &Store<Counter, Count>) -> i32 {
    let mut value = 0;
    store.read(&mut |state| value = state.value);
    value
}

#[test]
fn the_reducer_applies_each_action_to_state() {
    let store = store_with(vec![]);
    store.dispatch(Count::Increment);
    store.dispatch(Count::Add(5));
    assert_eq!(value_of(&store), 6);
}

#[test]
fn subscribers_fire_only_when_the_state_actually_changed() {
    let store = store_with(vec![]);
    let fires = Arc::new(Mutex::new(0));
    let observed = Arc::clone(&fires);
    store.subscribe(Box::new(move |_state| {
        *observed.lock().unwrap() += 1;
    }));

    store.dispatch(Count::Increment);
    store.dispatch(Count::Ignored);

    assert_eq!(*fires.lock().unwrap(), 1);
}

struct RecordAroundReducer {
    log: Arc<Mutex<Vec<String>>>,
}

impl Middleware<Counter, Count> for RecordAroundReducer {
    fn handle(&self, _api: &dyn StoreApi<Counter, Count>, next: &dyn Fn(Count), action: Count) {
        self.log
            .lock()
            .unwrap()
            .push("middleware-before".to_string());
        next(action);
        self.log
            .lock()
            .unwrap()
            .push("middleware-after".to_string());
    }
}

#[test]
fn middleware_wraps_the_reducer_which_is_the_terminal_link() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let store = store_with(vec![Arc::new(RecordAroundReducer {
        log: Arc::clone(&log),
    })]);
    let reduced_log = Arc::clone(&log);
    store.subscribe(Box::new(move |_state| {
        reduced_log.lock().unwrap().push("reducer-ran".to_string());
    }));

    store.dispatch(Count::Increment);

    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "middleware-before".to_string(),
            "reducer-ran".to_string(),
            "middleware-after".to_string(),
        ]
    );
}

struct ReinjectIncrement;

impl Middleware<Counter, Count> for ReinjectIncrement {
    fn handle(&self, api: &dyn StoreApi<Counter, Count>, next: &dyn Fn(Count), action: Count) {
        if matches!(action, Count::Add(_)) {
            api.dispatch(Count::Increment);
        }
        next(action);
    }
}

#[test]
fn api_dispatch_re_enters_the_chain_from_the_top() {
    let store = store_with(vec![Arc::new(ReinjectIncrement)]);
    store.dispatch(Count::Add(10));
    assert_eq!(value_of(&store), 11);
}
