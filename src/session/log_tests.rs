#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::log::*;
use crate::session::Message;
use crate::session::MessageType;
use chrono::Utc;
use tempfile::TempDir;
use tokio::sync::broadcast;

#[test]
fn appends_and_reads_messages() {
    let dir = TempDir::new().expect("temp dir");
    let msg1 = Message {
        from: "ryan:human".to_string(),
        text: "first message".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    };
    let msg2 = Message {
        from: "ryan:human".to_string(),
        text: "second message".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
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
fn creates_session_directory() {
    let base = TempDir::new().expect("temp base");
    let dir = create_session(base.path()).expect("create session");
    assert!(dir.exists());
    assert!(dir.is_dir());
    let current = base.path().join("current");
    assert!(current.is_symlink());
}

#[test]
fn resumes_existing_session() {
    let base = TempDir::new().expect("temp base");
    let dir1 = create_session(base.path()).expect("create session");
    assert!(dir1.exists());
    let msg = Message {
        from: "ryan:human".to_string(),
        text: "persisted message".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    };
    append_message(&dir1, &msg).expect("append");
    let dir2 = resume_or_create_session(base.path()).expect("resume");
    assert_eq!(dir1, dir2);
    let messages = read_messages(&dir2).expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "persisted message");
}

#[test]
fn session_broadcasts_and_reads() {
    let dir = TempDir::new().expect("temp dir");
    let (tx, mut rx) = broadcast::channel(16);
    let session = Session::new(dir.path().to_path_buf(), "test".to_string(), tx);
    let msg = Message {
        from: "ryan:human".to_string(),
        text: "via session".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    };
    session.broadcast(&msg).expect("broadcast");
    let received = rx.try_recv().expect("receive broadcast");
    assert_eq!(received.text, "via session");
    let messages = session.read_messages().expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "via session");
}

#[test]
fn session_derives_id_from_dir_name() {
    let base = TempDir::new().expect("temp base");
    let session_dir = base.path().join("my-session-id");
    std::fs::create_dir_all(&session_dir).expect("create dir");
    let (tx, _rx) = broadcast::channel(16);
    let session = Session::new(session_dir, "test".to_string(), tx);
    assert_eq!(session.id, "my-session-id");
}

#[test]
fn peer_count_shared_through_arc() {
    let dir = TempDir::new().expect("temp dir");
    let (tx, _rx) = broadcast::channel(16);
    let session = std::sync::Arc::new(Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let server_session = std::sync::Arc::clone(&session);

    assert_eq!(session.peer_count(), 0);

    server_session.peer_connected();
    assert_eq!(session.peer_count(), 1);

    server_session.peer_connected();
    assert_eq!(session.peer_count(), 2);

    server_session.peer_disconnected();
    assert_eq!(session.peer_count(), 1);

    server_session.peer_disconnected();
    assert_eq!(session.peer_count(), 0);
}
