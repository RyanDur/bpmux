use rusqlite::Connection;

use super::sqlite_engine as engine;

pub const RECENT_DESC_SQL: &str = "SELECT payload FROM messages ORDER BY seq DESC";

pub fn append(
    conn: &Connection,
    ts: &str,
    sender: &str,
    role: &str,
    recipient: Option<&str>,
    message_type: &str,
    payload: &str,
) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO messages (ts, sender, role, recipient, message_type, payload) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (ts, sender, role, recipient, message_type, payload),
    )
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
