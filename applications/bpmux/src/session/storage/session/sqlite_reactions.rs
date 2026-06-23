use rusqlite::Connection;

use super::sqlite_engine as engine;
use crate::session::reactions::ReactionsStore;

pub fn load(conn: &Connection) -> ReactionsStore {
    let mut store = ReactionsStore::default();
    let rows = engine::query(
        conn,
        "SELECT channel, reactions FROM reactions",
        [],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    if let Ok(rows) = rows {
        for (channel, raw) in rows {
            if let Ok(list) = serde_json::from_str(&raw) {
                store.channels.insert(channel, list);
            }
        }
    }
    store
}

pub fn replace(conn: &Connection, store: &ReactionsStore) -> Result<(), std::io::Error> {
    engine::execute(conn, "DELETE FROM reactions", []).map_err(engine::io_err)?;
    for (channel, list) in &store.channels {
        let raw = serde_json::to_string(list).map_err(std::io::Error::other)?;
        engine::execute(
            conn,
            "INSERT INTO reactions (channel, reactions) VALUES (?1, ?2)",
            (channel, &raw),
        )
        .map_err(engine::io_err)?;
    }
    Ok(())
}
