use rusqlite::Connection;

use database_support as engine;

pub struct ConnectionRow {
    pub owner: String,
    pub repo: String,
    pub base_url: Option<String>,
}

pub fn save_connection(conn: &Connection, row: &ConnectionRow) -> rusqlite::Result<()> {
    engine::execute(
        conn,
        "INSERT INTO github_connection (id, owner, repo, base_url) VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET owner = ?1, repo = ?2, base_url = ?3",
        (&row.owner, &row.repo, &row.base_url),
    )?;
    Ok(())
}

pub fn load_connection(conn: &Connection) -> rusqlite::Result<Option<ConnectionRow>> {
    engine::query_opt(
        conn,
        "SELECT owner, repo, base_url FROM github_connection WHERE id = 1",
        [],
        |row| {
            Ok(ConnectionRow {
                owner: row.get(0)?,
                repo: row.get(1)?,
                base_url: row.get(2)?,
            })
        },
    )
}

pub fn delete_connection(conn: &Connection) -> rusqlite::Result<()> {
    engine::execute(conn, "DELETE FROM github_connection WHERE id = 1", [])?;
    Ok(())
}
