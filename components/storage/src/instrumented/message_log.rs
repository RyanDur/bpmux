use std::time::Instant;

use super::InstrumentedStorage;
use crate::Storage;
use message::Message;
use message_log::LogError;

impl<S: Storage> message_log::MessageLog for InstrumentedStorage<S> {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let start = Instant::now();
        let result = self.inner.append_message(msg);
        self.log(&format!(
            "append_message {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let start = Instant::now();
        let result = self.inner.read_messages();
        if let Ok(ref msgs) = result {
            self.log(&format!(
                "read_messages {} msgs {:.1}ms",
                msgs.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        let start = Instant::now();
        let result = self.inner.read_recent(limit, offset, matches);
        if let Ok(ref msgs) = result {
            self.log(&format!(
                "read_recent {} msgs (limit {limit} offset {offset}) {:.1}ms",
                msgs.len(),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        result
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let start = Instant::now();
        let result = self.inner.clear_messages();
        self.log(&format!(
            "clear_messages {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }
}
