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

    fn search_hits(&self, needle_lower: &str) -> Result<Vec<u64>, LogError> {
        message_log::search_hits(&self.dir, needle_lower)
    }

    fn read_cluster(&self, center: u64, radius: u64) -> Result<Vec<(u64, Message)>, LogError> {
        message_log::read_cluster(&self.dir, center, radius)
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
        message_log::clear_messages(&self.dir)
    }
}
