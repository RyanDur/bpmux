use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Params, Row};

use crate::board::store::PersistError;

const DB_FILE: &str = "session.db";
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    #[test]
    fn migrations_are_forward_only_versioned() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).expect("migrations dir must exist") {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            count += 1;
            let versioned = name
                .strip_prefix('V')
                .and_then(|rest| rest.split_once("__"))
                .is_some_and(|(version, _)| {
                    !version.is_empty() && version.chars().all(|c| c.is_ascii_digit())
                });
            let migration_ext = name.ends_with(".sql") || name.ends_with(".rs");
            assert!(
                versioned && migration_ext,
                "migration `{name}` must be a forward versioned migration (V{{N}}__name.sql); refinery has no rollback/down — undo via a NEW V migration, never a U/down/rollback file"
            );
        }
        assert!(count > 0, "expected at least the V1 migration in {dir:?}");
    }
}
