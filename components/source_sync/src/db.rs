use std::collections::VecDeque;

use rusqlite::Connection;

use crate::QueuedEvent;
use database_support as engine;
use persist::PersistError;

pub fn load(conn: &Connection) -> Result<VecDeque<QueuedEvent>, PersistError> {
    Ok(engine::query(
        conn,
        "SELECT event FROM outbox ORDER BY seq ASC",
        [],
        |row| row.get::<_, String>(0),
    )
    .map_err(engine::persist_err)?
    .iter()
    .map(|raw| serde_json::from_str::<QueuedEvent>(raw).map_err(PersistError::from))
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .collect())
}

pub fn replace_rows(conn: &Connection, raws: &[String]) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM outbox", [])?;
    for raw in raws {
        conn.execute("INSERT INTO outbox (event) VALUES (?1)", [raw])?;
    }
    Ok(())
}

pub fn serialize_events(events: &[QueuedEvent]) -> Result<Vec<String>, PersistError> {
    events
        .iter()
        .map(|e| serde_json::to_string(e).map_err(PersistError::from))
        .collect()
}
