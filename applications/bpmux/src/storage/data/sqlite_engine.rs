use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Params, Row};

use crate::sync::PersistError;

const DB_FILE: &str = "data.db";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

mod embedded {
    refinery::embed_migrations!();
}

pub fn io_err(e: rusqlite::Error) -> std::io::Error {
    std::io::Error::other(e)
}

pub fn persist_err(e: rusqlite::Error) -> PersistError {
    PersistError::Io(io_err(e))
}

pub fn open(dir: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(dir.join(DB_FILE))?;
    let _journal_mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    embedded::migrations::runner().run(&mut conn).map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some(format!("refinery migration failed: {e}")),
        )
    })?;
    Ok(conn)
}

pub fn execute<P: Params>(conn: &Connection, sql: &str, params: P) -> rusqlite::Result<usize> {
    conn.execute(sql, params)
}

pub fn query<T, P, F>(conn: &Connection, sql: &str, params: P, map: F) -> rusqlite::Result<Vec<T>>
where
    P: Params,
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, map)?;
    rows.collect()
}

pub fn query_opt<T, P, F>(
    conn: &Connection,
    sql: &str,
    params: P,
    map: F,
) -> rusqlite::Result<Option<T>>
where
    P: Params,
    F: FnOnce(&Row<'_>) -> rusqlite::Result<T>,
{
    conn.query_row(sql, params, map).optional()
}
