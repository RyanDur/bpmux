use rusqlite::Connection;

use super::sqlite_engine as engine;

pub const OUTBOX_KEY: &str = "outbox";
pub const REACTIONS_KEY: &str = "reactions";

const STATE_PREFIX: &str = "state:";
const BOOKMARK_PREFIX: &str = "bookmark:";

pub fn state_key(key: &str) -> String {
    format!("{STATE_PREFIX}{key}")
}

pub fn bookmark_key(role: &str) -> String {
    format!("{BOOKMARK_PREFIX}{role}")
}

pub fn doc_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    engine::query_opt(
        conn,
        "SELECT value FROM documents WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
}

pub fn doc_put(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO documents (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
}

pub fn load_doc<T: serde::de::DeserializeOwned>(conn: &Connection, key: &str) -> Option<T> {
    let raw = doc_get(conn, key).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_doc<T: serde::Serialize>(
    conn: &Connection,
    key: &str,
    value: &T,
) -> Result<(), std::io::Error> {
    let raw = serde_json::to_string(value).map_err(std::io::Error::other)?;
    doc_put(conn, key, &raw).map(|_| ()).map_err(engine::io_err)
}
