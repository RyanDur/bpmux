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

    fn search_conversations(
        &self,
        needle_lower: &str,
        _addressee: Option<&str>,
        radius: u64,
        accept: &mut dyn FnMut(&[Message], usize) -> bool,
    ) -> Result<(), LogError> {
        let hits = message_log::search_hits(&self.dir, needle_lower)?;
        message_log::drive_search(
            hits,
            radius,
            |center, r| message_log::read_cluster(&self.dir, center, r),
            accept,
        )
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
