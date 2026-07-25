#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use super::SqliteStorage;
use super::import::import_conversation;
use message::{Message, MessageType};
use message_log::MessageLog;

fn chat_at(text: &str, rfc3339: &str) -> Message {
    let mut msg = Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: Utc.timestamp_opt(0, 0).unwrap(),
        message_type: MessageType::Chat,
        ..Message::default()
    };
    msg.ts = chrono::DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .with_timezone(&Utc);
    msg.ensure_id();
    msg
}

fn write_jsonl(dir: &TempDir, name: &str, messages: &[Message]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let lines: Vec<String> = messages
        .iter()
        .map(|m| serde_json::to_string(m).unwrap())
        .collect();
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    path
}

fn texts(storage: &SqliteStorage) -> Vec<String> {
    storage
        .read_messages()
        .unwrap()
        .into_iter()
        .map(|m| m.text)
        .collect()
}

#[test]
fn importing_older_history_interleaves_it_into_conversation_order() {
    let dir = TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    storage
        .append_message(&chat_at("local-tuesday", "2026-07-14T09:00:00Z"))
        .unwrap();
    storage
        .append_message(&chat_at("local-thursday", "2026-07-16T09:00:00Z"))
        .unwrap();

    let source = write_jsonl(
        &dir,
        "other-machine.jsonl",
        &[
            chat_at("imported-monday", "2026-07-13T09:00:00Z"),
            chat_at("imported-wednesday", "2026-07-15T09:00:00Z"),
        ],
    );

    let report = import_conversation(dir.path(), &source).unwrap();

    assert_eq!(report.imported, 2);
    assert_eq!(
        texts(&storage),
        vec![
            "imported-monday",
            "local-tuesday",
            "imported-wednesday",
            "local-thursday"
        ],
        "the merged conversation reads in created-at order, not arrival order"
    );
}

#[test]
fn importing_the_same_history_twice_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    storage
        .append_message(&chat_at("already-here", "2026-07-14T09:00:00Z"))
        .unwrap();

    let source = write_jsonl(
        &dir,
        "other-machine.jsonl",
        &[
            chat_at("new-arrival", "2026-07-13T09:00:00Z"),
            chat_at("id-less twin", "2026-07-13T10:00:00Z"),
        ],
    );

    let first = import_conversation(dir.path(), &source).unwrap();
    let second = import_conversation(dir.path(), &source).unwrap();

    assert_eq!(first.imported, 2);
    assert_eq!(second.imported, 0);
    assert_eq!(second.skipped_existing, 2);
    assert_eq!(
        texts(&storage),
        vec!["new-arrival", "id-less twin", "already-here"]
    );
}

#[test]
fn id_less_legacy_lines_import_once_and_dedupe_by_content() {
    let dir = TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();

    let mut legacy = chat_at("from the id-less era", "2026-04-03T09:00:00Z");
    legacy.id = String::new();
    let source = write_jsonl(&dir, "legacy.jsonl", &[legacy]);

    let first = import_conversation(dir.path(), &source).unwrap();
    let second = import_conversation(dir.path(), &source).unwrap();

    assert_eq!(first.imported, 1);
    assert_eq!(second.imported, 0);
    assert_eq!(texts(&storage), vec!["from the id-less era"]);
    let imported = &storage.read_messages().unwrap()[0];
    assert!(
        !imported.id.is_empty(),
        "the import mints the id the legacy line never had"
    );
}

#[test]
fn ephemeral_board_deltas_are_filtered_not_imported() {
    let dir = TempDir::new().unwrap();
    let storage = SqliteStorage::new(dir.path().to_path_buf()).unwrap();

    let mut delta = chat_at("", "2026-07-13T09:00:00Z");
    delta.message_type = MessageType::StateDelta;
    delta.state_delta = Some(message::StateDeltaPayload::Stories {
        stories: serde_json::json!([]),
        columns: None,
    });
    let source = write_jsonl(
        &dir,
        "noisy.jsonl",
        &[
            delta,
            chat_at("the real conversation", "2026-07-13T10:00:00Z"),
        ],
    );

    let report = import_conversation(dir.path(), &source).unwrap();

    assert_eq!(report.imported, 1);
    assert_eq!(report.filtered_ephemeral, 1);
    assert_eq!(texts(&storage), vec!["the real conversation"]);
}
