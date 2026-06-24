use rusqlite::Connection;

use super::sqlite_engine as engine;
use crate::log::LogError;
use crate::message::Message;

const RECENT_DESC_SQL: &str = "SELECT payload FROM messages ORDER BY seq DESC";

pub fn append(
    conn: &Connection,
    id: &str,
    created_at: i64,
    sender: Option<String>,
    message_type: &str,
    payload: &str,
) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO messages (id, created_at, sender, message_type, payload) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (id, created_at, sender, message_type, payload),
    )
}

pub fn read_recent(
    conn: &Connection,
    limit: usize,
    offset: usize,
    matches: &dyn Fn(&Message) -> bool,
) -> Result<Vec<Message>, LogError> {
    let mut stmt = conn.prepare(RECENT_DESC_SQL).map_err(engine::io_err)?;
    let mut rows = stmt.query([]).map_err(engine::io_err)?;
    let mut out = Vec::new();
    let mut to_skip = offset;
    while let Some(row) = rows.next().map_err(engine::io_err)? {
        let payload: String = row.get(0).map_err(engine::io_err)?;
        let msg: Message = serde_json::from_str(&payload)?;
        if matches(&msg) {
            if to_skip > 0 {
                to_skip -= 1;
                continue;
            }
            if out.len() >= limit {
                break;
            }
            out.push(msg);
        }
    }
    Ok(out)
}

pub fn all_payloads_asc(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    engine::query(
        conn,
        "SELECT payload FROM messages ORDER BY seq ASC",
        [],
        |row| row.get::<_, String>(0),
    )
}

pub fn clear(conn: &Connection) -> rusqlite::Result<usize> {
    engine::execute(conn, "DELETE FROM messages", [])
}
