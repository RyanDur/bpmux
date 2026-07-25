use super::SqliteStorage;
use super::sqlite_engine as engine;
use super::sqlite_messages as messages;
use message::Message;
use message_log::LogError;

const TAIL_MAX_BYTES: u64 = 8 * 1024 * 1024;

impl message_log::MessageLog for SqliteStorage {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let payload = serde_json::to_string(msg)?;
        let created_at = msg.ts.timestamp();
        let message_type = msg
            .message_type
            .to_wire()
            .unwrap_or_else(|| "chat".to_string());
        let conn = self.lock();
        let sender = self.resolve_sender(&conn, msg);
        messages::append(&conn, &msg.id, created_at, sender, &message_type, &payload)
            .map_err(engine::io_err)?;
        drop(conn);
        message_log::append_message(&self.dir, msg)?;
        if let Err(e) = message_log::rotate_if_larger_than(&self.dir, TAIL_MAX_BYTES) {
            diag::log_diag(&format!(
                "jsonl tail rotation failed ({e}); the tail keeps growing until it succeeds"
            ));
        }
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let payloads = messages::all_payloads_asc(&self.lock()).map_err(engine::io_err)?;
        payloads
            .iter()
            .map(|p| serde_json::from_str(p).map_err(LogError::from))
            .collect()
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        messages::read_recent(&self.lock(), limit, offset, matches)
    }

    fn search_conversations(
        &self,
        needle_lower: &str,
        radius: u64,
        accept: &mut dyn FnMut(&[Message], usize) -> bool,
    ) -> Result<(), LogError> {
        let hits = messages::search_hit_seqs(&self.lock(), needle_lower)
            .map_err(engine::io_err)
            .map_err(LogError::from)?;
        message_log::drive_search(
            hits,
            radius,
            |center, r| {
                let start = center.saturating_sub(r) as i64;
                let end = (center + r) as i64;
                let rows = messages::payloads_in_seq_range(&self.lock(), start, end)
                    .map_err(engine::io_err)?;
                rows.iter()
                    .map(|(seq, p)| {
                        serde_json::from_str(p)
                            .map(|m| (*seq as u64, m))
                            .map_err(LogError::from)
                    })
                    .collect()
            },
            accept,
        )
    }

    fn read_messages_since(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Message>, LogError> {
        let payloads = messages::payloads_since_asc(&self.lock(), cutoff.timestamp())
            .map_err(engine::io_err)?;
        payloads
            .iter()
            .map(|p| serde_json::from_str(p).map_err(LogError::from))
            .collect()
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        message_log::clear_messages(&self.dir)?;
        messages::clear(&self.lock())
            .map(|_| ())
            .map_err(engine::io_err)
    }
}
