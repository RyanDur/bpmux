use std::collections::HashSet;
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;

use crate::data_gateway::ConversationDataGateway;
use crate::db as messages;
use crate::{LogError, MessageLog};
use database_support::{DatabaseGateway, io_err};
use message::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped_existing: usize,
    pub filtered_ephemeral: usize,
    pub unparseable_lines: usize,
}

pub fn import_conversation(data_dir: &Path, source_jsonl: &Path) -> Result<ImportReport, LogError> {
    let gateway = Arc::new(DatabaseGateway::open(data_dir).map_err(io_err)?);
    user_routing::db::seed_system(&gateway.lock()).map_err(io_err)?;
    let target = ConversationDataGateway::new(gateway.clone(), data_dir.to_path_buf());
    let mut seen = known_identities(&target)?;
    let mut report = ImportReport {
        imported: 0,
        skipped_existing: 0,
        filtered_ephemeral: 0,
        unparseable_lines: 0,
    };

    let file = std::fs::File::open(source_jsonl)?;
    let conn = gateway.lock();
    conn.execute_batch("BEGIN IMMEDIATE").map_err(io_err)?;
    let inserted = (|| -> Result<(), LogError> {
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(mut msg) = serde_json::from_str::<Message>(&line) else {
                report.unparseable_lines += 1;
                continue;
            };
            if is_state_projection(&msg) {
                report.filtered_ephemeral += 1;
                continue;
            }
            if seen.contains(&content_key(&msg))
                || (!msg.id.is_empty() && seen.contains(&id_key(&msg)))
            {
                report.skipped_existing += 1;
                continue;
            }
            msg.ensure_id();
            seen.insert(id_key(&msg));
            seen.insert(content_key(&msg));
            let payload = serde_json::to_string(&msg)?;
            let message_type = msg
                .message_type
                .to_wire()
                .unwrap_or_else(|| "chat".to_string());
            let sender = target.resolve_sender(&conn, &msg);
            messages::append(
                &conn,
                &msg.id,
                msg.ts.timestamp(),
                sender,
                &message_type,
                &payload,
            )
            .map_err(io_err)?;
            messages::append_recipients(&conn, &msg.id, &msg).map_err(io_err)?;
            report.imported += 1;
        }
        Ok(())
    })();
    if let Err(e) = inserted {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(e);
    }
    conn.execute_batch("COMMIT").map_err(io_err)?;
    messages::restore_conversation_order(&conn).map_err(io_err)?;
    Ok(report)
}

fn known_identities(target: &ConversationDataGateway) -> Result<HashSet<String>, LogError> {
    let mut seen = HashSet::new();
    for msg in target.read_messages()? {
        if !msg.id.is_empty() {
            seen.insert(id_key(&msg));
        }
        seen.insert(content_key(&msg));
    }
    Ok(seen)
}

fn id_key(msg: &Message) -> String {
    format!("id:{}", msg.id)
}

fn content_key(msg: &Message) -> String {
    format!("content:{}:{}:{}", msg.ts.to_rfc3339(), msg.from, msg.text)
}

fn is_state_projection(msg: &Message) -> bool {
    matches!(
        msg.message_type,
        message::MessageType::StateDelta | message::MessageType::StateSnapshot
    )
}
