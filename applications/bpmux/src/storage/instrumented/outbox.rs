use std::time::Instant;

use super::InstrumentedStorage;
use crate::storage::Storage;

impl<S: Storage> crate::sync::OutboxRepo for InstrumentedStorage<S> {
    fn load_outbox(
        &self,
    ) -> Result<std::collections::VecDeque<crate::sync::QueuedEvent>, persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.load_outbox();
        if let Ok(ref queue) = result {
            self.log(&format!(
                "load_outbox {} events {:.1}ms",
                queue.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn save_outbox(
        &self,
        events: &[crate::sync::QueuedEvent],
    ) -> Result<(), persist::PersistError> {
        let start = Instant::now();
        let result = self.inner.save_outbox(events);
        self.log(&format!(
            "save_outbox {} events {:.1}ms",
            events.len(),
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }
}
