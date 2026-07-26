use message::Message;
use message_log::LogError;

use super::Hub;

impl Hub {
    pub fn broadcast(&self, msg: &Message) -> Result<(), LogError> {
        let mut msg = msg.clone();
        msg.ensure_id();
        self.storage.append_message(&msg)?;
        let _ = self.wake_tx.send(msg);
        Ok(())
    }

    pub fn broadcast_ephemeral(&self, msg: &Message) {
        let _ = self.wake_tx.send(msg.clone());
    }

    pub fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        self.storage.read_messages()
    }

    pub fn search_conversations(
        &self,
        needle_lower: &str,
        addressee: Option<&str>,
        radius: u64,
        accept: &mut dyn FnMut(&[Message], usize) -> bool,
    ) -> Result<(), LogError> {
        self.storage
            .search_conversations(needle_lower, addressee, radius, accept)
    }

    pub fn read_messages_since(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Message>, LogError> {
        self.storage.read_messages_since(cutoff)
    }

    pub fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        self.storage.read_recent(limit, offset, matches)
    }
}
