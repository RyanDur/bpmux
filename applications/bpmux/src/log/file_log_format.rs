#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::file_log::*;
use message::{Message, MessageType};

fn chat(text: &str) -> Message {
    Message {
        from: "dev".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    }
}

fn activity(text: &str) -> Message {
    Message {
        message_type: MessageType::Activity,
        ..chat(text)
    }
}

#[test]
fn read_recent_returns_newest_first_within_limit() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        append_message(dir.path(), &chat(&format!("m{i}"))).unwrap();
    }
    let recent = read_recent(dir.path(), 2, 0, &|_: &Message| true).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].text, "m4");
    assert_eq!(recent[1].text, "m3");
}

#[test]
fn read_recent_skips_offset_from_the_newest_end() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        append_message(dir.path(), &chat(&format!("m{i}"))).unwrap();
    }
    let page = read_recent(dir.path(), 2, 1, &|_: &Message| true).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].text, "m3");
    assert_eq!(page[1].text, "m2");
}

#[test]
fn read_recent_collects_limit_matches_skipping_non_matches() {
    let dir = tempfile::tempdir().unwrap();
    append_message(dir.path(), &chat("c1")).unwrap();
    append_message(dir.path(), &activity("a1")).unwrap();
    append_message(dir.path(), &chat("c2")).unwrap();
    append_message(dir.path(), &activity("a2")).unwrap();
    let recent = read_recent(dir.path(), 10, 0, &|m: &Message| m.is_chat()).unwrap();
    let texts: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["c2", "c1"]);
}

#[test]
fn read_recent_is_empty_when_no_log_exists() {
    let dir = tempfile::tempdir().unwrap();
    let recent = read_recent(dir.path(), 10, 0, &|_: &Message| true).unwrap();
    assert!(recent.is_empty());
}

#[test]
fn read_recent_crosses_the_reverse_read_chunk_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let padding = "x".repeat(1000);
    for i in 0..200 {
        append_message(dir.path(), &chat(&format!("m{i:04}-{padding}"))).unwrap();
    }
    let log_len = std::fs::metadata(log_path(dir.path())).unwrap().len();
    assert!(
        log_len > super::reverse_line_reader::REVERSE_READ_CHUNK,
        "fixture must exceed one reverse-read chunk to exercise the boundary"
    );
    let recent = read_recent(dir.path(), 3, 0, &|_: &Message| true).unwrap();
    assert_eq!(recent.len(), 3);
    assert!(recent[0].text.starts_with("m0199-"));
    assert!(recent[1].text.starts_with("m0198-"));
    assert!(recent[2].text.starts_with("m0197-"));
}

#[test]
fn read_recent_reads_a_final_line_with_no_trailing_newline() {
    let dir = tempfile::tempdir().unwrap();
    let a = serde_json::to_string(&chat("first")).unwrap();
    let b = serde_json::to_string(&chat("second")).unwrap();
    std::fs::write(log_path(dir.path()), format!("{a}\n{b}")).unwrap();
    let recent = read_recent(dir.path(), 10, 0, &|_: &Message| true).unwrap();
    let texts: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(texts, vec!["second", "first"]);
}

#[test]
fn read_recent_newest_first_equals_reversed_tail_of_full_read() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..20 {
        append_message(dir.path(), &chat(&format!("m{i}"))).unwrap();
    }
    let recent = read_recent(dir.path(), 5, 0, &|_: &Message| true).unwrap();
    let mut full = read_messages(dir.path()).unwrap();
    full.reverse();
    let from_full: Vec<&str> = full.iter().take(5).map(|m| m.text.as_str()).collect();
    let from_recent: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    assert_eq!(from_recent, from_full);
}
