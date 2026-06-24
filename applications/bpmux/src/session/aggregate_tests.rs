#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::aggregate::{BROADCAST_CHANNEL_CAPACITY, Session};
use crate::session::message::Message;
use chrono::Utc;
use tempfile::TempDir;
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

#[test]
fn cloned_session_starts_with_empty_roster() {
    let dir = TempDir::new().expect("temp dir");
    let (tx, _rx) = broadcast::channel(16);
    let session = Session::new(dir.path().to_path_buf(), "owner".to_string(), tx);
    session.roster_join("dev:claude-code");
    let cloned = session.clone();
    assert!(
        cloned.roster_snapshot().is_empty(),
        "clone must start with a fresh roster (snapshot semantics), not inherit live WS peers"
    );
}

#[test]
fn session_broadcasts_and_reads() {
    let dir = TempDir::new().expect("temp dir");
    let (tx, mut rx) = broadcast::channel(16);
    let session = Session::new(dir.path().to_path_buf(), "test".to_string(), tx);
    let msg = Message {
        from: "owner:human".to_string(),
        text: "via session".to_string(),
        ts: Utc::now(),
        ..Message::default()
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

fn burst_message(i: usize) -> Message {
    Message {
        from: "burst-producer".to_string(),
        text: format!("msg {i}"),
        ts: Utc::now(),
        ..Message::default()
    }
}

async fn run_burst_and_count_lag(capacity: usize, burst_size: usize) -> u64 {
    let (tx, mut rx) = broadcast::channel::<Message>(capacity);
    for i in 0..burst_size {
        tx.send(burst_message(i)).expect("send");
    }
    let mut lagged = 0u64;
    for _ in 0..burst_size {
        match rx.try_recv() {
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => lagged += n,
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
        }
    }
    lagged
}

#[tokio::test]
async fn broadcast_capacity_old_size_drops_under_large_burst() {
    let lagged = run_burst_and_count_lag(256, 500).await;
    assert!(
        lagged > 0,
        "baseline: 256-slot buffer should lag on a 500-message burst (got {lagged}); this test proves the capacity increase to {BROADCAST_CHANNEL_CAPACITY} is load-bearing"
    );
}

#[tokio::test]
async fn broadcast_capacity_new_size_survives_large_burst() {
    let lagged = run_burst_and_count_lag(BROADCAST_CHANNEL_CAPACITY, 500).await;
    assert_eq!(
        lagged, 0,
        "{BROADCAST_CHANNEL_CAPACITY}-slot buffer should not lag on a 500-message burst"
    );
}

#[tokio::test]
async fn broadcast_capacity_tolerates_burst_under_deliberating_subscriber() {
    let (tx, mut rx) = broadcast::channel::<Message>(BROADCAST_CHANNEL_CAPACITY);
    let producer = tokio::spawn({
        let tx = tx.clone();
        async move {
            for i in 0..50 {
                tx.send(burst_message(i)).expect("send");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    });
    let mut received = 0usize;
    let mut lagged = 0u64;
    for _ in 0..50 {
        match rx.recv().await {
            Ok(_) => {
                received += 1;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                lagged += n;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    producer.await.expect("producer join");
    assert_eq!(
        lagged, 0,
        "subscriber deliberating 500ms/msg through a 50-msg burst should not lag at capacity {BROADCAST_CHANNEL_CAPACITY}"
    );
    assert_eq!(received, 50, "should receive all 50 messages");
}
