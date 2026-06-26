use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Changed {
    Yes,
    No,
}

pub trait StoreApi<S, A> {
    fn dispatch(&self, action: A);
    fn read(&self, reader: &mut dyn FnMut(&S));
}

pub trait Middleware<S, A>: Send + Sync {
    fn handle(&self, api: &dyn StoreApi<S, A>, next: &dyn Fn(A), action: A);
}

type RootReducer<S, A> = Box<dyn Fn(&mut S, &A) -> Changed + Send + Sync>;
type Subscriber<S> = Box<dyn Fn(&S) + Send + Sync>;

pub struct Store<S, A> {
    state: Mutex<S>,
    reducer: RootReducer<S, A>,
    middleware: Vec<Arc<dyn Middleware<S, A>>>,
    subscribers: Mutex<Vec<Subscriber<S>>>,
}

impl<S, A> Store<S, A> {
    pub fn new(
        initial: S,
        reducer: RootReducer<S, A>,
        middleware: Vec<Arc<dyn Middleware<S, A>>>,
    ) -> Self {
        Self {
            state: Mutex::new(initial),
            reducer,
            middleware,
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self, subscriber: Subscriber<S>) {
        self.subscribers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(subscriber);
    }

    pub fn dispatch(&self, action: A) {
        self.dispatch_from(0, action);
    }

    pub fn read(&self, reader: &mut dyn FnMut(&S)) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reader(&state);
    }

    fn dispatch_from(&self, index: usize, action: A) {
        match self.middleware.get(index) {
            Some(link) => {
                let next = |forwarded: A| self.dispatch_from(index + 1, forwarded);
                link.handle(self, &next, action);
            }
            None => self.reduce_and_notify(action),
        }
    }

    fn reduce_and_notify(&self, action: A) {
        let changed = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (self.reducer)(&mut state, &action)
        };
        if changed == Changed::No {
            return;
        }
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let subscribers = self
            .subscribers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for subscriber in subscribers.iter() {
            subscriber(&state);
        }
    }
}

impl<S, A> StoreApi<S, A> for Store<S, A> {
    fn dispatch(&self, action: A) {
        self.dispatch_from(0, action);
    }

    fn read(&self, reader: &mut dyn FnMut(&S)) {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        reader(&state);
    }
}
