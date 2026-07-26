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
