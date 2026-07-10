#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::log::*;
use crate::message::Message;
use chrono::Utc;
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
