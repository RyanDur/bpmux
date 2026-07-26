#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::DatabaseGateway;

#[test]
fn opening_a_fresh_dir_builds_the_migrated_schema() {
    let dir = tempfile::TempDir::new().unwrap();
    let gateway = DatabaseGateway::open(&dir.path().join("nested/home")).unwrap();

    let tables: Vec<String> = super::query(
        &gateway.lock(),
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        [],
        |row| row.get(0),
    )
    .unwrap();

    for expected in ["messages", "message_recipients", "outbox", "users"] {
        assert!(
            tables.iter().any(|t| t == expected),
            "a fresh open replays every migration; missing {expected} in {tables:?}"
        );
    }
}

#[test]
fn reopening_the_same_dir_is_idempotent() {
    let dir = tempfile::TempDir::new().unwrap();
    drop(DatabaseGateway::open(dir.path()).unwrap());
    let gateway = DatabaseGateway::open(dir.path()).unwrap();

    let fk_on: i64 = gateway
        .lock()
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .unwrap();
    assert_eq!(fk_on, 1, "every open enforces foreign keys");
}

#[test]
fn a_failing_transaction_rolls_back_everything_it_wrote() {
    let dir = tempfile::TempDir::new().unwrap();
    let gateway = DatabaseGateway::open(dir.path()).unwrap();

    let result: Result<(), rusqlite::Error> = gateway.with_transaction(|conn| {
        super::execute(
            conn,
            "INSERT INTO meta (key, value) VALUES ('inside', 'the-transaction')",
            [],
        )?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM meta WHERE key = 'inside'", [], |r| {
                r.get(0)
            })?;
        assert_eq!(count, 1, "inside the transaction the write is visible");
        Err(rusqlite::Error::InvalidQuery)
    });
    assert!(result.is_err());

    let count: i64 = gateway
        .lock()
        .query_row("SELECT COUNT(*) FROM meta WHERE key = 'inside'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0, "the failure rolled the write back");
}

#[test]
fn a_successful_transaction_commits_atomically() {
    let dir = tempfile::TempDir::new().unwrap();
    let gateway = DatabaseGateway::open(dir.path()).unwrap();

    gateway
        .with_transaction(|conn| {
            super::execute(conn, "INSERT INTO meta (key, value) VALUES ('a', '1')", [])?;
            super::execute(conn, "INSERT INTO meta (key, value) VALUES ('b', '2')", [])
        })
        .unwrap();

    let count: i64 = gateway
        .lock()
        .query_row("SELECT COUNT(*) FROM meta", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "both writes landed together");
}

#[test]
fn a_transaction_inside_a_transaction_is_refused() {
    let dir = tempfile::TempDir::new().unwrap();
    let gateway = DatabaseGateway::open(dir.path()).unwrap();

    let result: Result<(), rusqlite::Error> = gateway.with_transaction(|conn| {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        Ok(())
    });
    assert!(
        result.is_err(),
        "starting a transaction when one is already in progress is an error, as in the reference"
    );
}
