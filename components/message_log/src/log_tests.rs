#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::*;
use chrono::Utc;
use message::Message;
use tempfile::TempDir;

#[test]
fn appends_and_reads_messages() {
    let dir = TempDir::new().expect("temp dir");
    let msg1 = Message {
        from: "owner:human".to_string(),
        text: "first message".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let msg2 = Message {
        from: "owner:human".to_string(),
        text: "second message".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    append_message(dir.path(), &msg1).expect("append 1");
    append_message(dir.path(), &msg2).expect("append 2");
    let messages = read_messages(dir.path()).expect("read");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text, "first message");
    assert_eq!(messages[1].text, "second message");
}

#[test]
fn reads_empty_when_no_log_exists() {
    let dir = TempDir::new().expect("temp dir");
    let messages = read_messages(dir.path()).expect("read");
    assert!(messages.is_empty());
}

#[test]
fn a_legacy_row_without_delivery_id_fields_decodes_with_none() {
    let dir = TempDir::new().expect("temp dir");
    let legacy_row = r#"{"from":"owner:human","text":"pre-B-1 row","ts":"2026-06-01T12:00:00Z"}"#;
    std::fs::write(log_path(dir.path()), format!("{legacy_row}\n")).expect("write legacy row");
    let messages = read_messages(dir.path()).expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender_id, None);
    assert_eq!(messages[0].to_ids, None);
    assert_eq!(messages[0].addressee_ids, None);
}

#[test]
fn read_messages_skips_zero_lines_with_legacy_state_delta_variant() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("messages-current.jsonl");
    let lines = [
        r#"{"from":"a","text":"first","ts":"2026-05-01T00:00:00Z"}"#,
        r#"{"from":"bpmux","text":"","ts":"2026-05-15T00:00:00Z","message_type":"state-delta","state_delta":{"domain":"icebox_migration","details":"retired"}}"#,
        r#"{"from":"b","text":"third","ts":"2026-06-01T00:00:00Z"}"#,
    ];
    std::fs::write(&path, lines.join("\n")).unwrap();
    let parsed = crate::read_messages(dir.path())
        .expect("read_messages tolerates legacy state-delta variants");
    assert_eq!(parsed.len(), 3, "all 3 lines parse — none dropped");
    assert!(parsed[1].state_delta.is_none(), "legacy variant → None");
}

#[test]
fn reading_since_a_cutoff_leaves_old_history_on_disk() {
    let dir = TempDir::new().expect("temp dir");
    let old = Message {
        from: "dev".to_string(),
        text: "#1 started long ago".to_string(),
        ts: Utc::now() - chrono::Duration::days(30),
        ..Message::default()
    };
    let recent_chat = Message {
        from: "dev".to_string(),
        text: "#2 started".to_string(),
        ts: Utc::now() - chrono::Duration::hours(2),
        ..Message::default()
    };
    let recent_activity = Message {
        from: "bpmux".to_string(),
        text: "#2 delivered".to_string(),
        message_type: message::MessageType::Activity,
        ts: Utc::now() - chrono::Duration::hours(1),
        ..Message::default()
    };
    append_message(dir.path(), &old).expect("append old");
    append_message(dir.path(), &recent_chat).expect("append recent chat");
    append_message(dir.path(), &recent_activity).expect("append recent activity");

    let cutoff = Utc::now() - chrono::Duration::days(14);
    let recent = read_messages_since(dir.path(), cutoff).expect("read since");

    let texts: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["#2 started", "#2 delivered"]);
}
