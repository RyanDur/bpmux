use std::collections::HashSet;

use rusqlite::Connection;

use super::sqlite_engine as engine;

pub fn load(conn: &Connection) -> HashSet<String> {
    engine::query(conn, "SELECT topic FROM surfaced_topics", [], |row| {
        row.get::<_, String>(0)
    })
    .unwrap_or_default()
    .into_iter()
    .collect()
}

pub fn replace(conn: &Connection, topics: &HashSet<String>) -> Result<(), std::io::Error> {
    let tx = conn.unchecked_transaction().map_err(engine::io_err)?;
    tx.execute("DELETE FROM surfaced_topics", [])
        .map_err(engine::io_err)?;
    for topic in topics {
        tx.execute("INSERT INTO surfaced_topics (topic) VALUES (?1)", [topic])
            .map_err(engine::io_err)?;
    }
    tx.commit().map_err(engine::io_err)?;
    Ok(())
}
