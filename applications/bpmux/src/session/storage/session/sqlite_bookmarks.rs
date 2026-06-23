use rusqlite::Connection;

use super::sqlite_engine as engine;
use super::sqlite_users as users;
use crate::session::bookmark::Bookmark;

pub fn get(conn: &Connection, role: &str) -> Bookmark {
    let Some(user_id) = users::id_for_role(conn, role).ok().flatten() else {
        return Bookmark::default();
    };
    engine::query_opt(
        conn,
        "SELECT bookmark FROM bookmarks WHERE user_id = ?1",
        [user_id],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .flatten()
    .and_then(|raw| serde_json::from_str(&raw).ok())
    .unwrap_or_default()
}

pub fn put(conn: &Connection, role: &str, bookmark: &Bookmark) -> Result<(), std::io::Error> {
    let Some(user_id) = users::id_for_role(conn, role).map_err(engine::io_err)? else {
        return Ok(());
    };
    let raw = serde_json::to_string(bookmark).map_err(std::io::Error::other)?;
    engine::execute(
        conn,
        "INSERT INTO bookmarks (user_id, bookmark) VALUES (?1, ?2) \
         ON CONFLICT(user_id) DO UPDATE SET bookmark = excluded.bookmark",
        (&user_id, &raw),
    )
    .map(|_| ())
    .map_err(engine::io_err)
}
