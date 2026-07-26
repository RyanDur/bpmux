#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::open;
use chrono::Utc;
use message::Message;

fn msg(text: &str) -> Message {
    let mut m = Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    m.ensure_id();
    m
}

#[test]
fn first_sqlite_boot_inherits_the_jsonl_history_and_stays_idempotent() {
    let dir = tempfile::TempDir::new().unwrap();
    message_log::append_message(dir.path(), &msg("from the file era")).unwrap();

    let storage = open(dir.path());
    let texts: Vec<String> = storage
        .read_messages()
        .unwrap()
        .iter()
        .map(|m| m.text.clone())
        .collect();
    assert_eq!(texts, vec!["from the file era".to_string()]);
    drop(storage);

    let again = open(dir.path());
    assert_eq!(
        again.read_messages().unwrap().len(),
        1,
        "a second boot must not double the history"
    );
}

#[test]
fn sqlite_appends_write_through_to_the_tail_the_plugin_hook_reads() {
    let dir = tempfile::TempDir::new().unwrap();
    let storage = open(dir.path());

    storage.append_message(&msg("visible to the hook")).unwrap();

    let tail = message_log::read_messages(dir.path()).unwrap();
    assert_eq!(tail.len(), 1, "the jsonl tail carries the new message");
    assert_eq!(tail[0].text, "visible to the hook");
}
