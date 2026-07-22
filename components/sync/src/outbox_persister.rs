use std::sync::Arc;

use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use source_sync::QueuedEvent;
use store::sync::queue_snapshot;
use store::{AppAction, AppState};

#[derive(Clone)]
pub enum OutboxSink {
    Storage(Arc<dyn storage::Storage>),
    #[cfg(test)]
    Recording(Arc<std::sync::Mutex<Vec<Vec<QueuedEvent>>>>),
}

impl OutboxSink {
    fn save(&self, events: &[QueuedEvent]) {
        let result = match self {
            OutboxSink::Storage(storage) => storage.save_outbox(events),
            #[cfg(test)]
            OutboxSink::Recording(log) => {
                log.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(events.to_vec());
                Ok(())
            }
        };
        if let Err(e) = result {
            diag::log_diag(&format!("sync: outbox persist failed: {e}"));
        }
    }
}

pub struct OutboxPersister {
    sink: OutboxSink,
}

impl OutboxPersister {
    pub fn new(sink: OutboxSink) -> Self {
        Self { sink }
    }
}

impl Listener<AppState, AppAction> for OutboxPersister {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.sync, &reaction.current.sync) {
            return Box::pin(async {});
        }
        let events = queue_snapshot(&reaction.current.sync);
        let sink = self.sink.clone();
        Box::pin(async move {
            sink.save(&events);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::LatestState
    }
}
