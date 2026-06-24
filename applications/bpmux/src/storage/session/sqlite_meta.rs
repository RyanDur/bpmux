use rusqlite::Connection;

use super::sqlite_engine as engine;

pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    engine::query_opt(
        conn,
        "SELECT value FROM meta WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
}

pub fn put(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO meta (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
}
