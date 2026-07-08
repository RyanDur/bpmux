use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use super::{Dispatch, Listener, Reaction};

pub(super) type LatestSlot<S, A> = watch::Sender<Option<(Arc<S>, A)>>;

pub(super) enum Delivery<S, A> {
    Ordered(mpsc::UnboundedSender<Reaction<S, A>>),
    Latest(LatestSlot<S, A>),
    FireAndForget(Arc<dyn Listener<S, A>>),
}

impl<S, A> Clone for Delivery<S, A> {
    fn clone(&self) -> Self {
        match self {
            Delivery::Ordered(sender) => Delivery::Ordered(sender.clone()),
            Delivery::Latest(sender) => Delivery::Latest(sender.clone()),
            Delivery::FireAndForget(listener) => Delivery::FireAndForget(Arc::clone(listener)),
        }
    }
}

pub(super) fn spawn_ordered<S, A>(
    listener: Arc<dyn Listener<S, A>>,
    mut inbox: mpsc::UnboundedReceiver<Reaction<S, A>>,
) where
    S: Send + Sync + 'static,
    A: Send + 'static,
{
    tokio::spawn(async move {
        while let Some(reaction) = inbox.recv().await {
            listener.on_action(reaction).await;
        }
    });
}

pub(super) fn spawn_latest<S, A>(
    listener: Arc<dyn Listener<S, A>>,
    mut slot: watch::Receiver<Option<(Arc<S>, A)>>,
    mut last_delivered: Arc<S>,
    dispatch: Dispatch<A>,
) where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
{
    tokio::spawn(async move {
        while slot.changed().await.is_ok() {
            let newest = slot.borrow_and_update().clone();
            let Some((current, action)) = newest else {
                continue;
            };
            listener
                .on_action(Reaction {
                    action,
                    previous: Arc::clone(&last_delivered),
                    current: Arc::clone(&current),
                    dispatch: Arc::clone(&dispatch),
                })
                .await;
            last_delivered = current;
        }
    });
}
