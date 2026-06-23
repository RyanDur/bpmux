use rusqlite::Connection;

use super::sqlite_engine as engine;
use crate::session::storage::UserRecord;

pub const SYSTEM_SESSION_ID: &str = "bpmux-system";

pub fn load_all(conn: &Connection) -> rusqlite::Result<Vec<UserRecord>> {
    engine::query(
        conn,
        "SELECT session_id, name, role, kind, last_delivered_at FROM users WHERE kind != 'system'",
        [],
        |row| {
            Ok(UserRecord {
                session_id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                kind: row.get(3)?,
                last_delivered_at: row.get(4)?,
            })
        },
    )
}

pub fn upsert(
    conn: &Connection,
    session_id: &str,
    name: &str,
    role: &str,
) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO users (id, session_id, name, role) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(session_id) DO UPDATE SET name = excluded.name, role = excluded.role, \
         updated_at = unixepoch()",
        (uuid::Uuid::new_v4().to_string(), session_id, name, role),
    )
}

pub fn set_last_delivered(conn: &Connection, session_id: &str, at: i64) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "UPDATE users SET last_delivered_at = ?2, updated_at = unixepoch() WHERE session_id = ?1",
        (session_id, at),
    )
}

pub fn delete(conn: &Connection, session_ids: &[String]) -> rusqlite::Result<usize> {
    let mut affected = 0;
    for session_id in session_ids {
        affected += engine::execute(
            conn,
            "DELETE FROM users WHERE session_id = ?1",
            (session_id,),
        )?;
    }
    Ok(affected)
}

pub fn id_for_session(conn: &Connection, session_id: &str) -> rusqlite::Result<Option<String>> {
    engine::query_opt(
        conn,
        "SELECT id FROM users WHERE session_id = ?1",
        (session_id,),
        |row| row.get(0),
    )
}

pub fn id_for_role(conn: &Connection, role: &str) -> rusqlite::Result<Option<String>> {
    engine::query_opt(
        conn,
        "SELECT id FROM users WHERE role = ?1 ORDER BY updated_at DESC LIMIT 1",
        (role,),
        |row| row.get(0),
    )
}

pub fn seed_system(conn: &Connection) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO users (id, session_id, name, role, kind) \
         VALUES (?1, ?2, 'bpmux', 'system', 'system') \
         ON CONFLICT(session_id) DO NOTHING",
        (uuid::Uuid::new_v4().to_string(), SYSTEM_SESSION_ID),
    )
}
