use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;

pub type ListenerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Dispatch<A> = Arc<dyn Fn(A) + Send + Sync>;

pub struct Reaction<S, A> {
    pub action: A,
    pub previous: Arc<S>,
    pub current: Arc<S>,
    pub dispatch: Dispatch<A>,
}

pub trait Listener<S, A>: Send + Sync {
    fn on_action(&self, reaction: Reaction<S, A>) -> ListenerFuture;
}

type RootReducer<S, A> = Box<dyn Fn(&S, &A) -> S + Send + Sync>;

pub struct Store<S, A> {
    state: Mutex<Arc<S>>,
    reducer: RootReducer<S, A>,
    listeners: Mutex<Vec<Arc<dyn Listener<S, A>>>>,
    in_flight: Mutex<Vec<JoinHandle<()>>>,
}

impl<S, A> Store<S, A>
where
    S: Send + Sync + 'static,
    A: Clone + Send + 'static,
{
    pub fn new(initial: S, reducer: RootReducer<S, A>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Arc::new(initial)),
            reducer,
            listeners: Mutex::new(Vec::new()),
            in_flight: Mutex::new(Vec::new()),
        })
    }

    pub fn add_listener(&self, listener: Arc<dyn Listener<S, A>>) {
        self.listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(listener);
    }

    pub fn select<T>(&self, selector: impl FnOnce(&S) -> T) -> T {
        let snapshot = Arc::clone(
            &self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        selector(snapshot.as_ref())
    }

    pub fn dispatch(self: &Arc<Self>, action: impl Into<A>) {
        let action = action.into();
        let (previous, current) = {
            let mut guard = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = Arc::clone(&guard);
            let current = Arc::new((self.reducer)(previous.as_ref(), &action));
            *guard = Arc::clone(&current);
            (previous, current)
        };

        let dispatch = self.dispatcher();
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        let mut in_flight = self
            .in_flight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        in_flight.retain(|handle| !handle.is_finished());
        for listener in listeners {
            let reaction = Reaction {
                action: action.clone(),
                previous: Arc::clone(&previous),
                current: Arc::clone(&current),
                dispatch: Arc::clone(&dispatch),
            };
            in_flight.push(tokio::spawn(listener.on_action(reaction)));
        }
    }

    fn dispatcher(self: &Arc<Self>) -> Dispatch<A> {
        let weak = Arc::downgrade(self);
        Arc::new(move |action: A| {
            if let Some(store) = weak.upgrade() {
                store.dispatch(action);
            }
        })
    }

    pub async fn await_listeners(&self) {
        loop {
            let batch: Vec<JoinHandle<()>> = {
                let mut in_flight = self
                    .in_flight
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                std::mem::take(&mut *in_flight)
            };
            if batch.is_empty() {
                break;
            }
            for handle in batch {
                let _ = handle.await;
            }
        }
    }
}
