#![allow(clippy::unwrap_used)]

use super::session::{BROADCAST_CHANNEL_CAPACITY, Session};
use crate::session::message::Message;
use tokio::sync::broadcast;

#[test]
fn broadcast_mints_and_persists_a_message_id() {
    let dir = tempfile::tempdir().unwrap();
    let (wake_tx, _rx) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    let session = Session::new(dir.path().to_path_buf(), "test".to_string(), wake_tx);
    let msg = Message {
        from: "dev".to_string(),
        text: "hi".to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    };
    assert!(msg.id.is_empty());
    session.broadcast(&msg).unwrap();
    let stored = session.read_messages().unwrap();
    assert_eq!(stored.len(), 1);
    assert!(
        !stored[0].id.is_empty(),
        "broadcast stamps a server-authoritative id before persisting and sending"
    );
}
