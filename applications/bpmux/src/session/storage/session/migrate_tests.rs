#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use chrono::Utc;
use tempfile::TempDir;

use super::migrate::{migrate_file_log_to_sqlite, migrate_routing_identity_to_agents};
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

fn routing_layout(base: &TempDir) -> std::path::PathBuf {
    let session_dir = base.path().join("current");
    std::fs::create_dir_all(&session_dir).unwrap();
    session_dir
}

fn seed_routing_json(base: &TempDir, entries: &[(&str, &str, &str)]) {
    let display: std::collections::HashMap<&str, &str> =
        entries.iter().map(|(id, d, _)| (*id, *d)).collect();
    let cursor: std::collections::HashMap<&str, &str> =
        entries.iter().map(|(id, _, c)| (*id, *c)).collect();
    let json = serde_json::json!({ "display_by_id": display, "cursor_by_id": cursor });
    std::fs::write(
        base.path().join("routing_identity.json"),
        serde_json::to_string(&json).unwrap(),
    )
    .unwrap();
}

#[test]
fn migrates_legacy_routing_json_display_and_durable_cursor_into_the_agents_table() {
    let base = TempDir::new().unwrap();
    let session_dir = routing_layout(&base);
    seed_routing_json(
        &base,
        &[
            ("sess-dev", "dev", "2026-06-20T10:00:00Z"),
            ("sess-pm", "pm:claude-code", "2026-06-20T11:00:00Z"),
        ],
    );

    let report = migrate_routing_identity_to_agents(&session_dir).unwrap();
    assert_eq!(report.migrated, 2);
    assert!(!report.already_complete);

    let agents = SqliteStorage::new(session_dir.clone())
        .unwrap()
        .load_agents()
        .unwrap();
    let dev = agents.iter().find(|a| a.id == "sess-dev").unwrap();
    assert_eq!(
        dev.name, "dev",
        "the opaque display migrates as the agent name"
    );
    assert_eq!(dev.role, "dev", "the role-col is short(display)");
    assert_eq!(
        dev.cursor.as_deref(),
        Some("2026-06-20T10:00:00Z"),
        "the durable drain cursor is carried across the flip — so the agent resumes from where it left off and never re-floods already-seen history"
    );
    let pm = agents.iter().find(|a| a.id == "sess-pm").unwrap();
    assert_eq!(
        pm.role, "pm",
        "the role-col is short(display) even for a full id:client display"
    );
    assert_eq!(pm.cursor.as_deref(), Some("2026-06-20T11:00:00Z"));
}

#[test]
fn routing_migration_is_idempotent_and_never_reverts_a_post_flip_advanced_cursor() {
    let base = TempDir::new().unwrap();
    let session_dir = routing_layout(&base);
    seed_routing_json(&base, &[("sess-dev", "dev", "2026-06-20T10:00:00Z")]);

    let first = migrate_routing_identity_to_agents(&session_dir).unwrap();
    assert_eq!(first.migrated, 1);
    assert!(!first.already_complete);

    let storage = SqliteStorage::new(session_dir.clone()).unwrap();
    storage
        .set_agent_cursor("sess-dev", "2026-06-25T00:00:00Z")
        .unwrap();

    let second = migrate_routing_identity_to_agents(&session_dir).unwrap();
    assert!(
        second.already_complete,
        "a completed routing migration re-runs as a no-op"
    );
    assert_eq!(second.migrated, 0);

    let dev = storage
        .load_agents()
        .unwrap()
        .into_iter()
        .find(|a| a.id == "sess-dev")
        .unwrap();
    assert_eq!(
        dev.cursor.as_deref(),
        Some("2026-06-25T00:00:00Z"),
        "the cursor advanced by real post-flip drains is NEVER reverted to the stale JSON value — the migration flag makes the re-run a no-op, so running migrate-storage twice can't clobber live drain positions"
    );
}

#[test]
fn routing_migration_with_no_legacy_json_is_a_clean_noop() {
    let base = TempDir::new().unwrap();
    let session_dir = routing_layout(&base);

    let report = migrate_routing_identity_to_agents(&session_dir).unwrap();
    assert_eq!(
        report.migrated, 0,
        "a fresh install with no legacy routing_identity.json migrates nothing — a clean no-op, never an error"
    );
    assert!(!report.already_complete);
    assert!(
        SqliteStorage::new(session_dir)
            .unwrap()
            .load_agents()
            .unwrap()
            .is_empty(),
        "no JSON ⇒ no agent rows fabricated"
    );
}
