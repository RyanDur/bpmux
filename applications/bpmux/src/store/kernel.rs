use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

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

pub trait Middleware<S, A>: Send + Sync {
    fn transform(&self, state: &S, action: A) -> A;
}

type RootReducer<S, A> = Box<dyn Fn(&S, &A) -> S + Send + Sync>;

pub struct Store<S, A> {
    state: Mutex<Arc<S>>,
    middleware: Mutex<Vec<Arc<dyn Middleware<S, A>>>>,
    reducer: RootReducer<S, A>,
    listeners: Mutex<Vec<Arc<dyn Listener<S, A>>>>,
}

impl<S, A> Store<S, A>
where
    S: Send + Sync + 'static,
    A: Clone + Send + 'static,
{
    pub fn new(initial: S, reducer: RootReducer<S, A>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Arc::new(initial)),
            middleware: Mutex::new(Vec::new()),
            reducer,
            listeners: Mutex::new(Vec::new()),
        })
    }

    pub fn add_middleware(&self, middleware: Arc<dyn Middleware<S, A>>) {
        self.middleware
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(middleware);
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
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        for listener in listeners {
            let reaction = Reaction {
                action: action.clone(),
                previous: Arc::clone(&previous),
                current: Arc::clone(&current),
                dispatch: Arc::clone(&dispatch),
            };
            tokio::spawn(listener.on_action(reaction));
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
