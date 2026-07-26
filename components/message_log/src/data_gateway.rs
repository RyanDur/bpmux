use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;

use crate::db as messages;
use crate::{LogError, MessageLog, drive_search};
use database_support::{DatabaseGateway, io_err};
use message::Message;

const TAIL_MAX_BYTES: u64 = 8 * 1024 * 1024;

pub struct ConversationDataGateway {
    gateway: Arc<DatabaseGateway>,
    dir: PathBuf,
}

impl ConversationDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>, dir: PathBuf) -> Self {
        Self { gateway, dir }
    }

    pub(crate) fn resolve_sender(&self, conn: &Connection, msg: &Message) -> Option<String> {
        let user_id = if user_routing::short_name(&msg.from) == "bpmux" {
            user_routing::db::SYSTEM_USER_ID
        } else {
            msg.sender_id.as_deref()?
        };
        user_routing::db::id_for_user(conn, user_id).ok().flatten()
    }
}

impl MessageLog for ConversationDataGateway {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        let payload = serde_json::to_string(msg)?;
        let created_at = msg.ts.timestamp();
        let message_type = msg
            .message_type
            .to_wire()
            .unwrap_or_else(|| "chat".to_string());
        let conn = self.gateway.lock();
        let sender = self.resolve_sender(&conn, msg);
        messages::append(&conn, &msg.id, created_at, sender, &message_type, &payload)
            .map_err(io_err)?;
        messages::append_recipients(&conn, &msg.id, msg).map_err(io_err)?;
        drop(conn);
        crate::append_message(&self.dir, msg)?;
        if let Err(e) = crate::rotate_if_larger_than(&self.dir, TAIL_MAX_BYTES) {
            diag::log_diag(&format!(
                "jsonl tail rotation failed ({e}); the tail keeps growing until it succeeds"
            ));
        }
        Ok(())
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        let payloads = messages::all_payloads_asc(&self.gateway.lock()).map_err(io_err)?;
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
        messages::read_recent(&self.gateway.lock(), limit, offset, matches)
    }

    fn search_conversations(
        &self,
        needle_lower: &str,
        addressee: Option<&str>,
        radius: u64,
        accept: &mut dyn FnMut(&[Message], usize) -> bool,
    ) -> Result<(), LogError> {
        let hits = messages::search_hit_seqs(&self.gateway.lock(), needle_lower, addressee)
            .map_err(io_err)
            .map_err(LogError::from)?;
        drive_search(
            hits,
            radius,
            |center, r| {
                let start = center.saturating_sub(r) as i64;
                let end = (center + r) as i64;
                let rows = messages::payloads_in_seq_range(&self.gateway.lock(), start, end)
                    .map_err(io_err)?;
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
        let payloads = messages::payloads_since_asc(&self.gateway.lock(), cutoff.timestamp())
            .map_err(io_err)?;
        payloads
            .iter()
            .map(|p| serde_json::from_str(p).map_err(LogError::from))
            .collect()
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        crate::clear_messages(&self.dir)?;
        messages::clear(&self.gateway.lock())
            .map(|_| ())
            .map_err(io_err)
    }
}
