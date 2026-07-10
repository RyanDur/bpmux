use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, watch};

mod worker;

use worker::{Delivery, spawn_latest, spawn_ordered};

pub type ListenerFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type Dispatch<A> = Arc<dyn Fn(A) + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeliveryMode {
    Ordered,
    LatestState,
    FireAndForget,
}

pub struct Reaction<S, A> {
    pub action: A,
    pub previous: Arc<S>,
    pub current: Arc<S>,
    pub dispatch: Dispatch<A>,
}

pub trait Listener<S, A>: Send + Sync {
    fn on_action(&self, reaction: Reaction<S, A>) -> ListenerFuture;

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub trait Middleware<S, A>: Send + Sync {
    fn transform(&self, state: &S, action: A) -> A;
}

type RootReducer<S, A> = Box<dyn Fn(&S, &A) -> S + Send + Sync>;

pub struct Store<S, A> {
    state: Mutex<Arc<S>>,
    middleware: Mutex<Vec<Arc<dyn Middleware<S, A>>>>,
    reducer: RootReducer<S, A>,
    deliveries: Mutex<Vec<Delivery<S, A>>>,
}

impl<S, A> Store<S, A>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    pub fn new(initial: S, reducer: RootReducer<S, A>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Arc::new(initial)),
            middleware: Mutex::new(Vec::new()),
            reducer,
            deliveries: Mutex::new(Vec::new()),
        })
    }

    pub fn add_middleware(&self, middleware: Arc<dyn Middleware<S, A>>) {
        self.middleware
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(middleware);
    }

    pub fn add_listener(self: &Arc<Self>, listener: Arc<dyn Listener<S, A>>) {
        let delivery = match listener.delivery_mode() {
            DeliveryMode::Ordered => {
                let (sender, inbox) = mpsc::unbounded_channel();
                spawn_ordered(listener, inbox);
                Delivery::Ordered(sender)
            }
            DeliveryMode::LatestState => {
                let (sender, slot) = watch::channel(None);
                spawn_latest(listener, slot, self.snapshot(), self.dispatcher());
                Delivery::Latest(sender)
            }
            DeliveryMode::FireAndForget => Delivery::FireAndForget(listener),
        };
        self.deliveries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(delivery);
    }

    pub fn select<T>(&self, selector: impl FnOnce(&S) -> T) -> T {
        selector(self.snapshot().as_ref())
    }

    fn snapshot(&self) -> Arc<S> {
        Arc::clone(
            &self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    pub fn dispatch(self: &Arc<Self>, action: impl Into<A>) {
        let action = action.into();
        let middleware = self
            .middleware
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let (action, previous, current) = {
            let mut guard = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = Arc::clone(&guard);
            let action = middleware.iter().fold(action, |action, link| {
                link.transform(previous.as_ref(), action)
            });
            let current = Arc::new((self.reducer)(previous.as_ref(), &action));
            *guard = Arc::clone(&current);
            (action, previous, current)
        };

        let dispatch = self.dispatcher();
        let deliveries = self
            .deliveries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        for delivery in deliveries {
            match delivery {
                Delivery::Ordered(sender) => {
                    let _ = sender.send(Reaction {
                        action: action.clone(),
                        previous: Arc::clone(&previous),
                        current: Arc::clone(&current),
                        dispatch: Arc::clone(&dispatch),
                    });
                }
                Delivery::Latest(sender) => {
                    let _ = sender.send(Some((Arc::clone(&current), action.clone())));
                }
                Delivery::FireAndForget(listener) => {
                    tokio::spawn(listener.on_action(Reaction {
                        action: action.clone(),
                        previous: Arc::clone(&previous),
                        current: Arc::clone(&current),
                        dispatch: Arc::clone(&dispatch),
                    }));
                }
            }
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
}

#[cfg(test)]
mod kernel_tests;
