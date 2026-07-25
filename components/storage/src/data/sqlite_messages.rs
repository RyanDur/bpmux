use rusqlite::Connection;

use super::sqlite_engine as engine;
use message::Message;
use message_log::LogError;

const RECENT_DESC_SQL: &str = "SELECT payload FROM messages ORDER BY seq DESC";
const SINCE_ASC_SQL: &str = "SELECT payload FROM messages WHERE created_at >= ?1 ORDER BY seq ASC";

pub fn search_hit_seqs(conn: &Connection, needle_lower: &str) -> rusqlite::Result<Vec<u64>> {
    let mut stmt = conn.prepare(
        "SELECT seq FROM messages WHERE LOWER(payload) LIKE '%' || ?1 || '%' ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map([needle_lower], |row| row.get::<_, i64>(0))?;
    rows.map(|r| r.map(|s| s as u64)).collect()
}

pub fn payloads_in_seq_range(
    conn: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT seq, payload FROM messages WHERE seq BETWEEN ?1 AND ?2 ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map([start, end], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn payloads_since_asc(conn: &Connection, cutoff: i64) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(SINCE_ASC_SQL)?;
    let rows = stmt.query_map([cutoff], |row| row.get(0))?;
    rows.collect()
}

pub fn append(
    conn: &Connection,
    id: &str,
    created_at: i64,
    sender: Option<String>,
    message_type: &str,
    payload: &str,
) -> rusqlite::Result<usize> {
    engine::execute(
        conn,
        "INSERT INTO messages (id, created_at, sender, message_type, payload) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (id, created_at, sender, message_type, payload),
    )
}

pub fn read_recent(
    conn: &Connection,
    limit: usize,
    offset: usize,
    matches: &dyn Fn(&Message) -> bool,
) -> Result<Vec<Message>, LogError> {
    let mut stmt = conn.prepare(RECENT_DESC_SQL).map_err(engine::io_err)?;
    let mut rows = stmt.query([]).map_err(engine::io_err)?;
    let mut out = Vec::new();
    let mut to_skip = offset;
    while let Some(row) = rows.next().map_err(engine::io_err)? {
        let payload: String = row.get(0).map_err(engine::io_err)?;
        let msg: Message = serde_json::from_str(&payload)?;
        if matches(&msg) {
            if to_skip > 0 {
                to_skip -= 1;
                continue;
            }
            if out.len() >= limit {
                break;
            }
            out.push(msg);
        }
    }
    Ok(out)
}

pub fn all_payloads_asc(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    engine::query(
        conn,
        "SELECT payload FROM messages ORDER BY seq ASC",
        [],
        |row| row.get::<_, String>(0),
    )
}

pub fn clear(conn: &Connection) -> rusqlite::Result<usize> {
    engine::execute(conn, "DELETE FROM messages", [])
}
