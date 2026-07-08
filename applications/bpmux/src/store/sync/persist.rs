use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::Mutex;

use crate::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::app::{AppAction, AppState};
use crate::sync::QueuedEvent;

use super::queue_snapshot;

#[derive(Clone)]
pub enum OutboxSink {
    Storage(Arc<dyn crate::storage::Storage>),
    Path(PathBuf),
}

impl OutboxSink {
    fn save(&self, events: &[QueuedEvent]) {
        let result = match self {
            OutboxSink::Storage(storage) => storage.save_outbox(events),
            OutboxSink::Path(path) => crate::sync::persist_outbox_to_disk(path, events),
        };
        if let Err(e) = result {
            crate::diag::log_diag(&format!("sync: outbox persist failed: {e}"));
        }
    }
}

pub struct OutboxPersister {
    sink: OutboxSink,
    next_seq: AtomicU64,
    last_saved: Arc<Mutex<u64>>,
}

impl OutboxPersister {
    pub fn new(sink: OutboxSink) -> Self {
        Self {
            sink,
            next_seq: AtomicU64::new(0),
            last_saved: Arc::new(Mutex::new(0)),
        }
    }
}

impl Listener<AppState, AppAction> for OutboxPersister {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.sync, &reaction.current.sync) {
            return Box::pin(async {});
        }
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let events = queue_snapshot(&reaction.current.sync);
        let sink = self.sink.clone();
        let last_saved = Arc::clone(&self.last_saved);
        Box::pin(async move {
            let mut last_saved = last_saved.lock().await;
            if seq < *last_saved {
                return;
            }
            sink.save(&events);
            *last_saved = seq;
        })
    }
}
