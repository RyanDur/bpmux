use rusqlite::Connection;

use super::sqlite_engine as engine;
use crate::session::bookmark::Bookmark;

pub fn get(conn: &Connection, role: &str) -> Bookmark {
    engine::query_opt(
        conn,
        "SELECT bookmark FROM bookmarks WHERE role = ?1",
        [role],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .flatten()
    .and_then(|raw| serde_json::from_str(&raw).ok())
    .unwrap_or_default()
}

pub fn put(conn: &Connection, role: &str, bookmark: &Bookmark) -> Result<(), std::io::Error> {
    let raw = serde_json::to_string(bookmark).map_err(std::io::Error::other)?;
    engine::execute(
        conn,
        "INSERT INTO bookmarks (role, bookmark) VALUES (?1, ?2) \
         ON CONFLICT(role) DO UPDATE SET bookmark = excluded.bookmark",
        [role, &raw],
    )
    .map(|_| ())
    .map_err(engine::io_err)
}
