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
