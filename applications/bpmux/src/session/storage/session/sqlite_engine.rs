use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};

use crate::board::store::PersistError;

const DB_FILE: &str = "session.db";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub const RECENT_DESC_SQL: &str = "SELECT payload FROM messages ORDER BY seq DESC";

pub fn io_err(e: rusqlite::Error) -> std::io::Error {
    std::io::Error::other(e)
}

pub fn persist_err(e: rusqlite::Error) -> PersistError {
    PersistError::Io(io_err(e))
}

pub fn open(dir: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(dir.join(DB_FILE))?;
    let _journal_mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS messages (
             seq INTEGER PRIMARY KEY AUTOINCREMENT,
             payload TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS documents (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )?;
    Ok(conn)
}

pub fn append_message(conn: &Connection, payload: &str) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO messages (payload) VALUES (?1)", [payload])?;
    Ok(())
}

pub fn all_payloads_asc(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT payload FROM messages ORDER BY seq ASC")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

pub fn clear_messages(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM messages", [])?;
    Ok(())
}

pub fn doc_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM documents WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
}

pub fn doc_put(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO documents (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

pub fn doc_keys_with_prefix(conn: &Connection, prefix: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT key FROM documents WHERE key LIKE ?1")?;
    let pattern = format!("{prefix}%");
    let rows = stmt.query_map([pattern.as_str()], |row| row.get::<_, String>(0))?;
    rows.collect()
}
