use rusqlite::Connection;

use database_support as engine;

pub fn record_if_changed(
    conn: &Connection,
    run_id: u64,
    state: &str,
    observed_at: i64,
) -> rusqlite::Result<bool> {
    let last: Option<String> = engine::query_opt(
        conn,
        "SELECT state FROM ci_runs WHERE run_id = ?1 ORDER BY seq DESC LIMIT 1",
        [run_id as i64],
        |row| row.get(0),
    )?;
    if last.as_deref() == Some(state) {
        return Ok(false);
    }
    engine::execute(
        conn,
        "INSERT INTO ci_runs (run_id, state, observed_at) VALUES (?1, ?2, ?3)",
        (run_id as i64, state, observed_at),
    )?;
    Ok(true)
}

pub fn since(conn: &Connection, cutoff: i64) -> rusqlite::Result<Vec<(u64, String, i64)>> {
    engine::query(
        conn,
        "SELECT run_id, state, observed_at FROM ci_runs WHERE observed_at >= ?1 ORDER BY seq ASC",
        [cutoff],
        |row| {
            Ok((
                row.get::<_, i64>(0)? as u64,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )
}
