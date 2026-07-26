use rusqlite::Connection;

use database_support as engine;

pub fn record_if_changed(
    conn: &Connection,
    story_id: u64,
    new_state: &str,
    observed_at: i64,
) -> rusqlite::Result<bool> {
    let last: Option<String> = engine::query_opt(
        conn,
        "SELECT new_state FROM story_transitions WHERE story_id = ?1 ORDER BY seq DESC LIMIT 1",
        [story_id as i64],
        |row| row.get(0),
    )?;
    if last.as_deref() == Some(new_state) {
        return Ok(false);
    }
    engine::execute(
        conn,
        "INSERT INTO story_transitions (story_id, new_state, observed_at) VALUES (?1, ?2, ?3)",
        (story_id as i64, new_state, observed_at),
    )?;
    Ok(true)
}

pub fn since(conn: &Connection, cutoff: i64) -> rusqlite::Result<Vec<(u64, String, i64)>> {
    engine::query(
        conn,
        "SELECT story_id, new_state, observed_at FROM story_transitions \
         WHERE observed_at >= ?1 ORDER BY seq ASC",
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
