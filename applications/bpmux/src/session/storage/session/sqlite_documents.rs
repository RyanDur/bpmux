use rusqlite::Connection;

use super::sqlite_engine as engine;

pub const LESSONS_KEY: &str = "lessons";
pub const BOARD_SNAPSHOT_KEY: &str = "board_snapshot";
pub const OUTBOX_KEY: &str = "outbox";
pub const REACTIONS_KEY: &str = "reactions";

const PEER_LESSONS_PREFIX: &str = "peer_lessons:";
const STATE_PREFIX: &str = "state:";
const BOOKMARK_PREFIX: &str = "bookmark:";

pub fn peer_key(peer: &str) -> String {
    format!("{PEER_LESSONS_PREFIX}{peer}")
}

pub fn state_key(key: &str) -> String {
    format!("{STATE_PREFIX}{key}")
}

pub fn bookmark_key(role: &str) -> String {
    format!("{BOOKMARK_PREFIX}{role}")
}

pub fn load_doc<T: serde::de::DeserializeOwned>(conn: &Connection, key: &str) -> Option<T> {
    let raw = engine::doc_get(conn, key).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_doc<T: serde::Serialize>(
    conn: &Connection,
    key: &str,
    value: &T,
) -> Result<(), std::io::Error> {
    let raw = serde_json::to_string(value).map_err(std::io::Error::other)?;
    engine::doc_put(conn, key, &raw).map_err(engine::io_err)
}

pub fn list_peer_names(conn: &Connection) -> Vec<String> {
    engine::doc_keys_with_prefix(conn, PEER_LESSONS_PREFIX)
        .unwrap_or_default()
        .iter()
        .filter_map(|k| k.strip_prefix(PEER_LESSONS_PREFIX).map(str::to_string))
        .collect()
}
