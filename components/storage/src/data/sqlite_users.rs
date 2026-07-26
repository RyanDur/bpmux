use rusqlite::Connection;

use database_support as engine;
use user_routing::UserRecord;

pub const SYSTEM_USER_ID: &str = "bpmux-system";

pub fn load_all(conn: &Connection) -> rusqlite::Result<Vec<UserRecord>> {
    engine::query(
        conn,
        "SELECT user_id, name, role, kind, last_delivered_at FROM users WHERE kind != 'system'",
        [],
        |row| {
            Ok(UserRecord {
                user_id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                kind: row.get(3)?,
                last_delivered_at: row.get(4)?,
            })
        },
    )
}

pub fn upsert(conn: &Connection, user_id: &str, name: &str, role: &str) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO users (id, user_id, name, role) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(user_id) DO UPDATE SET name = excluded.name, role = excluded.role, \
         updated_at = unixepoch()",
        (uuid::Uuid::new_v4().to_string(), user_id, name, role),
    )
}

pub fn set_last_delivered(conn: &Connection, user_id: &str, at: i64) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "UPDATE users SET last_delivered_at = ?2, updated_at = unixepoch() WHERE user_id = ?1",
        (user_id, at),
    )
}

pub fn delete(conn: &Connection, user_ids: &[String]) -> rusqlite::Result<usize> {
    let mut affected = 0;
    for user_id in user_ids {
        affected += engine::execute(conn, "DELETE FROM users WHERE user_id = ?1", (user_id,))?;
    }
    Ok(affected)
}

pub fn id_for_user(conn: &Connection, user_id: &str) -> rusqlite::Result<Option<String>> {
    engine::query_opt(
        conn,
        "SELECT id FROM users WHERE user_id = ?1",
        (user_id,),
        |row| row.get(0),
    )
}

pub fn seed_system(conn: &Connection) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO users (id, user_id, name, role, kind) \
         VALUES (?1, ?2, 'bpmux', 'system', 'system') \
         ON CONFLICT(user_id) DO NOTHING",
        (uuid::Uuid::new_v4().to_string(), SYSTEM_USER_ID),
    )
}
