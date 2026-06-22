#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chrono::Utc;
use tempfile::TempDir;

use super::migrate::migrate_file_log_to_sqlite;
use super::{FileStorage, SqliteStorage, Storage};
use crate::session::message::{Message, MessageType, StateDeltaPayload};

fn chat(text: &str) -> Message {
    Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    }
}

fn delta(payload: StateDeltaPayload) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(payload),
        ..Message::default()
    }
}

fn board_snapshot() -> Message {
    delta(StateDeltaPayload::BoardSnapshot {
        stories: serde_json::json!([]),
    })
}

fn is_board_snapshot(msg: &Message) -> bool {
    matches!(
        msg.state_delta,
        Some(StateDeltaPayload::BoardSnapshot { .. })
    )
}

fn seed_jsonl(dir: &Path, msgs: &[Message]) {
    let source = FileStorage::new(dir.to_path_buf());
    for m in msgs {
        source.append_message(m).unwrap();
    }
}

fn read_target(dir: &Path) -> Vec<Message> {
    SqliteStorage::new(dir.to_path_buf())
        .unwrap()
        .read_messages()
        .unwrap()
}

#[test]
fn drops_only_board_snapshot_keeps_all_else_in_order() {
    let dir = TempDir::new().unwrap();
    seed_jsonl(
        dir.path(),
        &[
            chat("first"),
            board_snapshot(),
            delta(StateDeltaPayload::Roster { members: vec![] }),
            chat("second"),
            delta(StateDeltaPayload::BoardInvalidated {
                reason: "board changed".to_string(),
            }),
            board_snapshot(),
            chat("third"),
        ],
    );

    let report = migrate_file_log_to_sqlite(dir.path()).unwrap();
    assert_eq!(report.filtered, 2, "both BoardSnapshot deltas dropped");
    assert_eq!(
        report.migrated, 5,
        "chat x3 + Roster + BoardInvalidated migrated (roster is durable per #1642; BoardInvalidated is a replay-event)"
    );

    let migrated = read_target(dir.path());
    assert!(
        migrated.iter().all(|m| !is_board_snapshot(m)),
        "zero BoardSnapshot deltas survive in the migrated store"
    );
    assert_eq!(
        migrated
            .iter()
            .filter(|m| matches!(m.state_delta, Some(StateDeltaPayload::Roster { .. })))
            .count(),
        1,
        "roster delta is preserved (durable content, not bloat)"
    );
    assert_eq!(
        migrated
            .iter()
            .filter(|m| matches!(
                m.state_delta,
                Some(StateDeltaPayload::BoardInvalidated { .. })
            ))
            .count(),
        1,
        "BoardInvalidated is a replay-event, preserved"
    );
    let chat_order: Vec<String> = migrated
        .iter()
        .filter(|m| m.message_type == MessageType::Chat)
        .map(|m| m.text.clone())
        .collect();
    assert_eq!(
        chat_order,
        vec!["first", "second", "third"],
        "chronological file-order preserved across the migration (cursor-replay safe)"
    );
}

#[test]
fn rerun_is_idempotent_and_adds_no_rows() {
    let dir = TempDir::new().unwrap();
    seed_jsonl(dir.path(), &[chat("a"), board_snapshot(), chat("b")]);

    let first = migrate_file_log_to_sqlite(dir.path()).unwrap();
    assert_eq!(first.migrated, 2);
    assert!(!first.already_complete);

    let second = migrate_file_log_to_sqlite(dir.path()).unwrap();
    assert!(
        second.already_complete,
        "a completed migration re-runs as a no-op"
    );
    assert_eq!(second.migrated, 0);

    assert_eq!(
        read_target(dir.path()).len(),
        2,
        "re-running the migration must not duplicate rows"
    );
}

#[test]
fn recovers_from_a_partial_interrupted_run() {
    let dir = TempDir::new().unwrap();
    seed_jsonl(dir.path(), &[chat("a"), board_snapshot(), chat("b")]);

    let partial = SqliteStorage::new(dir.path().to_path_buf()).unwrap();
    partial
        .append_message(&chat("leftover-partial-row"))
        .unwrap();
    drop(partial);

    let report = migrate_file_log_to_sqlite(dir.path()).unwrap();
    assert!(!report.already_complete);
    assert_eq!(report.migrated, 2);

    let migrated = read_target(dir.path());
    let texts: Vec<String> = migrated.iter().map(|m| m.text.clone()).collect();
    assert_eq!(
        texts,
        vec!["a", "b"],
        "the stale partial row is cleared; the store converges to exactly the source survivors"
    );
}
