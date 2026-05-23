#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

use super::peer::{PeerMessage, spawn_relay};
use crate::session::{Message, MessageType, Session};

fn make_session() -> (Arc<Session>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, rx) = broadcast::channel(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    (session, rx, dir)
}

async fn next_system_text(rx: &mut broadcast::Receiver<Message>) -> Option<String> {
    loop {
        match timeout(Duration::from_secs(2), rx.recv())
            .await
            .ok()?
            .ok()?
        {
            m if matches!(m.message_type, MessageType::System) => return Some(m.text),
            _ => continue,
        }
    }
}

#[tokio::test]
async fn hello_produces_joined_system_message() {
    let (session, mut rx, _dir) = make_session();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        session.clone(),
        "remote".to_string(),
        "local-session".to_string(),
        inbound_rx,
        outbound_tx,
    );

    let hello = PeerMessage::Hello {
        session_id: "remote-session".to_string(),
        name: "alice".to_string(),
    };
    inbound_tx
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();

    let text = next_system_text(&mut rx)
        .await
        .expect("no system message received");
    assert_eq!(text, "alice joined");
}

#[tokio::test]
async fn inbound_close_produces_left_system_message() {
    let (session, mut rx, _dir) = make_session();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        session.clone(),
        "remote".to_string(),
        "local-session".to_string(),
        inbound_rx,
        outbound_tx,
    );

    let hello = PeerMessage::Hello {
        session_id: "remote-session".to_string(),
        name: "alice".to_string(),
    };
    inbound_tx
        .send(serde_json::to_string(&hello).unwrap())
        .await
        .unwrap();

    let joined = next_system_text(&mut rx).await.expect("joined");
    assert_eq!(joined, "alice joined");

    drop(inbound_tx);

    let text = next_system_text(&mut rx)
        .await
        .expect("no left message received");
    assert_eq!(text, "alice left");
}

#[tokio::test]
async fn left_uses_placeholder_name_when_no_hello_received() {
    let (session, mut rx, _dir) = make_session();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        session.clone(),
        "remote".to_string(),
        "local-session".to_string(),
        inbound_rx,
        outbound_tx,
    );

    drop(inbound_tx);

    let text = next_system_text(&mut rx)
        .await
        .expect("no left message received");
    assert_eq!(text, "remote left");
}
