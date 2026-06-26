#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::sqlite::SqliteStorage;
use crate::message::{Message, MessageType};
use crate::storage::Storage;

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

#[test]
fn a_bookmark_is_reachable_by_role_and_cascades_when_the_user_is_deleted() {
    let (_dir, storage) = store();
    storage.upsert_user("sess-dev", "dev", "dev").unwrap();
    let bm = crate::bookmark::Bookmark {
        current_task: "wire the bookmark FK".to_string(),
        ..Default::default()
    };
    storage.save_bookmark("dev", &bm).unwrap();
    assert_eq!(
        storage.load_bookmark("dev").current_task,
        "wire the bookmark FK",
        "a bookmark is reachable by role, resolved through to the user's id"
    );

    storage.delete_users(&["sess-dev".to_string()]).unwrap();
    assert_eq!(
        storage.load_bookmark("dev"),
        crate::bookmark::Bookmark::default(),
        "ON DELETE CASCADE removes the bookmark when its user row is reaped — load_bookmark falls back to the default, observable through the public Storage API"
    );
}
