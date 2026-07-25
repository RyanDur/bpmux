use super::FileStorage;
use message::Message;
use message_log::LogError;

impl message_log::MessageLog for FileStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        message_log::append_message(&self.dir, msg)
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        message_log::read_messages(&self.dir)
    }

    fn read_messages_since(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Message>, LogError> {
        message_log::read_messages_since(&self.dir, cutoff)
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        message_log::read_recent(&self.dir, limit, offset, matches)
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        let log_path = self.dir.join("log.jsonl");
        match std::fs::remove_file(&log_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}
