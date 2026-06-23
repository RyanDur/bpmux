#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::session::message::{Message, MessageType};

fn store() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    (dir, storage)
}

fn chat(from: &str, sender_id: Option<&str>) -> Message {
    Message {
        from: from.to_string(),
        sender_id: sender_id.map(str::to_string),
        text: "hi".to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    }
}

fn senders(storage: &SqliteStorage) -> Vec<Option<String>> {
    engine::query(
        &storage.lock(),
        "SELECT sender FROM messages ORDER BY seq",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn seeds_the_system_user_on_open_but_excludes_it_from_the_participant_roster() {
    let (_dir, storage) = store();
    assert!(
        users::id_for_session(&storage.lock(), users::SYSTEM_SESSION_ID)
            .unwrap()
            .is_some(),
        "the system user is seeded at store open so server-originated messages have an FK target"
    );
    assert!(
        storage.load_users().unwrap().is_empty(),
        "the system user is infrastructure, not a routable participant — load_users (which rebuilds routing identities) excludes it, so the backends stay at parity"
    );
}

#[test]
fn a_message_from_a_registered_user_stores_that_users_surrogate_id_as_sender() {
    let (_dir, storage) = store();
    storage
        .upsert_user("sess-dev", "dev:claude-code", "dev")
        .unwrap();
    let dev_id = users::id_for_session(&storage.lock(), "sess-dev")
        .unwrap()
        .expect("the registered user has a surrogate id");

    storage
        .append_message(&chat("dev:claude-code", Some("sess-dev")))
        .unwrap();

    assert_eq!(
        senders(&storage),
        vec![Some(dev_id)],
        "the sender FK holds the resolved user's surrogate id, not a denormalized name"
    );
}

#[test]
fn a_server_originated_message_attributes_to_the_seeded_system_user() {
    let (_dir, storage) = store();
    let system_id = users::id_for_session(&storage.lock(), users::SYSTEM_SESSION_ID)
        .unwrap()
        .expect("the system user has a surrogate id");

    storage.append_message(&chat("bpmux", None)).unwrap();

    assert_eq!(
        senders(&storage),
        vec![Some(system_id)],
        "a message from the server resolves to the system user — bpmux IS a user, not an unknown sender"
    );
}

#[test]
fn an_unregistered_sender_persists_as_null_never_a_dropped_message() {
    let (_dir, storage) = store();
    storage.append_message(&chat("rando", None)).unwrap();
    assert_eq!(
        senders(&storage),
        vec![None],
        "an author the store cannot resolve persists as a NULL sender — the message is kept, never dropped to satisfy the FK"
    );
}

#[test]
fn the_sender_foreign_key_is_enforced_so_a_dangling_reference_cannot_be_written() {
    let (_dir, storage) = store();
    let dangling = engine::execute(
        &storage.lock(),
        "INSERT INTO messages (created_at, sender, message_type, payload) VALUES (1, 'no-such-user-uuid', 'chat', '{}')",
        [],
    );
    assert!(
        dangling.is_err(),
        "foreign_keys=ON rejects a sender that references no user row — the FK has teeth"
    );
}

#[test]
fn a_bookmark_keys_by_the_resolved_user_id_and_cascades_when_the_user_is_deleted() {
    let (_dir, storage) = store();
    storage.upsert_user("sess-dev", "dev", "dev").unwrap();
    let bm = crate::session::bookmark::Bookmark {
        current_task: "wire the bookmark FK".to_string(),
        ..Default::default()
    };
    storage.save_bookmark("dev", &bm).unwrap();
    assert_eq!(
        storage.load_bookmark("dev").current_task,
        "wire the bookmark FK",
        "a bookmark is reachable by role, resolved through to the user's id"
    );

    let dev_id = users::id_for_role(&storage.lock(), "dev")
        .unwrap()
        .expect("the dev user resolves to an id");
    let keyed_by_user: Option<String> = engine::query_opt(
        &storage.lock(),
        "SELECT bookmark FROM bookmarks WHERE user_id = ?1",
        [&dev_id],
        |row| row.get(0),
    )
    .unwrap();
    assert!(
        keyed_by_user.is_some(),
        "the bookmark row is keyed by the user's uuid, not the bare role string"
    );

    storage.delete_users(&["sess-dev".to_string()]).unwrap();
    let remaining: i64 = engine::query_opt(
        &storage.lock(),
        "SELECT count(*) FROM bookmarks",
        [],
        |row| row.get(0),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        remaining, 0,
        "ON DELETE CASCADE removes the bookmark when its user row is reaped"
    );
}
