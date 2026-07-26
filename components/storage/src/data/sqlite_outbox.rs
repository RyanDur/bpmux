use std::collections::VecDeque;

use rusqlite::Connection;

use database_support as engine;
use persist::PersistError;
use source_sync::QueuedEvent;

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

pub fn replace(conn: &Connection, events: &[QueuedEvent]) -> Result<(), PersistError> {
    let raws = events
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let tx = conn.unchecked_transaction().map_err(engine::persist_err)?;
    tx.execute("DELETE FROM outbox", [])
        .map_err(engine::persist_err)?;
    for raw in &raws {
        tx.execute("INSERT INTO outbox (event) VALUES (?1)", [raw])
            .map_err(engine::persist_err)?;
    }
    tx.commit().map_err(engine::persist_err)?;
    Ok(())
}
