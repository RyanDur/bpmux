use std::collections::HashSet;
use std::io::BufRead;
use std::path::Path;

use super::SqliteStorage;
use super::migrate::MigrationError;
use message::Message;
use message_log::MessageLog;
use message_log::db as messages;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped_existing: usize,
    pub filtered_ephemeral: usize,
    pub unparseable_lines: usize,
}

pub fn import_conversation(
    data_dir: &Path,
    source_jsonl: &Path,
) -> Result<ImportReport, MigrationError> {
    let target = SqliteStorage::new(data_dir.to_path_buf())?;
    let mut seen = known_identities(&target)?;
    let mut report = ImportReport {
        imported: 0,
        skipped_existing: 0,
        filtered_ephemeral: 0,
        unparseable_lines: 0,
    };

    let file = std::fs::File::open(source_jsonl)
        .map_err(|e| MigrationError::Store(message_log::LogError::from(e)))?;
    let conn = target.lock();
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let inserted = (|| -> Result<(), MigrationError> {
        for line in std::io::BufReader::new(file).lines() {
            let line = line.map_err(|e| MigrationError::Store(message_log::LogError::from(e)))?;
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
            let payload = serde_json::to_string(&msg).map_err(message_log::LogError::from)?;
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
            )?;
            messages::append_recipients(&conn, &msg.id, &msg)?;
            report.imported += 1;
        }
        Ok(())
    })();
    if let Err(e) = inserted {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(e);
    }
    conn.execute_batch("COMMIT")?;
    messages::restore_conversation_order(&conn).map_err(MigrationError::OpenTarget)?;
    Ok(report)
}

fn known_identities(target: &SqliteStorage) -> Result<HashSet<String>, MigrationError> {
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
