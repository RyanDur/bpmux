#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::sqlite::SqliteStorage;
use message::{Message, MessageType};
use message_log::MessageLog;
use user_routing::UserRepo;

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

const FIX_QUEUED_VERDICT_ROW: &str = r#"{"id":0,"event":{"VerdictCast":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"verdict":{"Fail":{"step":"acceptance"}},"payload":"narrow viewport"}}}"#;

#[test]
fn an_outbox_row_round_trips_byte_identically_through_the_real_store() {
    use source_sync::OutboxRepo;
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("data.db")).unwrap();
    raw.execute(
        "INSERT INTO outbox (event) VALUES (?1)",
        [FIX_QUEUED_VERDICT_ROW],
    )
    .unwrap();

    let loaded = storage.load_outbox().unwrap();
    assert_eq!(loaded.len(), 1);
    let entries: Vec<source_sync::QueuedEvent> = loaded.into_iter().collect();
    storage.save_outbox(&entries).unwrap();

    let stored: String = raw
        .query_row("SELECT event FROM outbox", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        stored, FIX_QUEUED_VERDICT_ROW,
        "the SQLite outbox row survives load + save byte-identically — SQLite has no legacy fallback, so this byte-freeze is its only protection"
    );
}

#[test]
fn reading_since_a_cutoff_serves_the_window_from_the_indexed_column() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();

    let mut old = chat("dev", None);
    old.text = "#1 started long ago".to_string();
    old.ts = chrono::Utc::now() - chrono::Duration::days(30);
    let mut recent = chat("dev", None);
    recent.text = "#2 started".to_string();
    recent.ts = chrono::Utc::now() - chrono::Duration::hours(2);
    let mut recent_activity = Message {
        from: "bpmux".to_string(),
        text: "#2 delivered".to_string(),
        ts: chrono::Utc::now() - chrono::Duration::hours(1),
        message_type: MessageType::Activity,
        ..Message::default()
    };
    recent_activity.ensure_id();

    storage.append_message(&old).unwrap();
    storage.append_message(&recent).unwrap();
    storage.append_message(&recent_activity).unwrap();

    let cutoff = chrono::Utc::now() - chrono::Duration::days(14);
    let window = storage.read_messages_since(cutoff).unwrap();

    let texts: Vec<&str> = window.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["#2 started", "#2 delivered"]);
}

#[test]
fn search_hits_and_clusters_reconstruct_the_conversation_neighborhood() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();

    for text in ["opening remarks", "the needle sits here", "closing remarks"] {
        let mut m = chat("dev", None);
        m.text = text.to_string();
        storage.append_message(&m).unwrap();
    }

    let hits = storage.search_hits("needle").unwrap();
    assert_eq!(hits.len(), 1, "one message carries the needle");

    let cluster = storage.read_cluster(hits[0], 5).unwrap();
    let texts: Vec<&str> = cluster.iter().map(|(_, m)| m.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["opening remarks", "the needle sits here", "closing remarks"],
        "the cluster carries the neighborhood in order"
    );
    let (center_ordinal, center) = cluster
        .iter()
        .find(|(o, _)| *o == hits[0])
        .expect("the hit ordinal resolves inside its own cluster");
    assert_eq!(center.text, "the needle sits here");
    assert_eq!(*center_ordinal, hits[0]);
}
