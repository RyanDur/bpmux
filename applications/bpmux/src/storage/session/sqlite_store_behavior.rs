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
