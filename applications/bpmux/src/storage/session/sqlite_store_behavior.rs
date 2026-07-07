#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::sqlite::SqliteStorage;
use crate::log::MessageLog;
use crate::message::{Message, MessageType};
use crate::routing::UserRepo;

fn store() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    (dir, storage)
}

fn chat(from: &str, sender_id: Option<&str>) -> Message {
    let mut msg = Message {
        from: from.to_string(),
        sender_id: sender_id.map(str::to_string),
        text: "hi".to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    };
    msg.ensure_id();
    msg
}

#[test]
fn the_seeded_system_user_is_excluded_from_the_participant_roster() {
    let (_dir, storage) = store();
    assert!(
        storage.load_users().unwrap().is_empty(),
        "the system user is infrastructure, not a routable participant — load_users (which rebuilds routing identities) excludes it, so the backends stay at parity"
    );
}

#[test]
fn an_unresolvable_author_is_kept_never_dropped() {
    let (_dir, storage) = store();
    storage.append_message(&chat("rando", None)).unwrap();
    let stored = storage.read_messages().unwrap();
    assert_eq!(
        stored.len(),
        1,
        "an author the store cannot resolve is still persisted — the message is kept, never dropped to satisfy the FK"
    );
    assert_eq!(
        stored[0].from, "rando",
        "the kept message round-trips its original author verbatim"
    );
}

const FIX_QUEUED_VERDICT_ROW: &str = r#"{"id":0,"event":{"VerdictCast":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"verdict":{"Fail":{"step":"acceptance"}},"payload":"narrow viewport"}},"pre_state":null,"last_classification":null}"#;

#[test]
fn an_outbox_row_round_trips_byte_identically_through_the_real_store() {
    use crate::sync::OutboxRepo;
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("session.db")).unwrap();
    raw.execute(
        "INSERT INTO outbox (event) VALUES (?1)",
        [FIX_QUEUED_VERDICT_ROW],
    )
    .unwrap();

    let loaded = storage.load_outbox().unwrap();
    assert_eq!(loaded.len(), 1);
    let entries: Vec<crate::sync::QueuedEvent> = loaded.into_iter().collect();
    storage.save_outbox(&entries).unwrap();

    let stored: String = raw
        .query_row("SELECT event FROM outbox", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        stored, FIX_QUEUED_VERDICT_ROW,
        "the SQLite outbox row survives load + save byte-identically — SQLite has no legacy fallback, so this byte-freeze is its only protection"
    );
}
