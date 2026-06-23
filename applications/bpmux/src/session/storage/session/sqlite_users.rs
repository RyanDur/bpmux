use rusqlite::Connection;

use super::sqlite_engine as engine;
use crate::session::storage::UserRecord;

pub fn load_all(conn: &Connection) -> rusqlite::Result<Vec<UserRecord>> {
    engine::query(
        conn,
        "SELECT id, name, role, kind, cursor FROM users",
        [],
        |row| {
            Ok(UserRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                kind: row.get(3)?,
                cursor: row.get(4)?,
            })
        },
    )
}

pub fn upsert(conn: &Connection, id: &str, name: &str, role: &str) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO users (id, name, role) VALUES (?1, ?2, ?3) \
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, role = excluded.role, \
         updated_at = unixepoch()",
        (id, name, role),
    )
}

pub fn set_cursor(conn: &Connection, id: &str, cursor: &str) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "UPDATE users SET cursor = ?2, updated_at = unixepoch() WHERE id = ?1",
        (id, cursor),
    )
}

pub fn delete(conn: &Connection, ids: &[String]) -> rusqlite::Result<usize> {
    let mut affected = 0;
    for id in ids {
        affected += engine::execute(conn, "DELETE FROM users WHERE id = ?1", (id,))?;
    }
    Ok(affected)
}
