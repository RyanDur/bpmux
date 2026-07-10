#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

use super::peer::{PeerMessage, spawn_relay};
use crate::hub::Hub;
use message::{Message, MessageType};

fn make_hub() -> (Arc<Hub>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, rx) = broadcast::channel(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    (hub, rx, dir)
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
    let (hub, mut rx, _dir) = make_hub();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        hub.clone(),
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
    let (hub, mut rx, _dir) = make_hub();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        hub.clone(),
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
async fn chat_relay_round_trips_delivery_id_fields_outbound_then_inbound() {
    let (hub_a, _rx_a, _dir_a) = make_hub();
    let (_inbound_tx_a, inbound_rx_a) = mpsc::channel::<String>(16);
    let (outbound_tx_a, mut outbound_rx_a) = mpsc::channel::<String>(16);
    let _peer_a = spawn_relay(
        hub_a.clone(),
        "remote".to_string(),
        "session-a".to_string(),
        inbound_rx_a,
        outbound_tx_a,
    );

    let msg = Message {
        from: "dev:claude-code".to_string(),
        sender_id: Some("id-dev".to_string()),
        text: "cross-peer ids".to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["pm".to_string()]),
        to_ids: Some(vec!["id-pm".to_string()]),
        addressee: Some("pm".to_string()),
        addressee_ids: Some(vec!["id-pm".to_string()]),
        ..Message::default()
    };
    hub_a.broadcast(&msg).unwrap();

    let outbound_json = timeout(Duration::from_secs(2), outbound_rx_a.recv())
        .await
        .expect("no outbound peer frame")
        .expect("outbound channel closed");

    let (hub_b, mut rx_b, _dir_b) = make_hub();
    let (inbound_tx_b, inbound_rx_b) = mpsc::channel::<String>(16);
    let (outbound_tx_b, _outbound_rx_b) = mpsc::channel::<String>(16);
    let _peer_b = spawn_relay(
        hub_b.clone(),
        "remote-a".to_string(),
        "session-b".to_string(),
        inbound_rx_b,
        outbound_tx_b,
    );
    inbound_tx_b
        .send(outbound_json)
        .await
        .expect("feed outbound frame to a different peer session");

    let received = loop {
        match timeout(Duration::from_secs(2), rx_b.recv())
            .await
            .expect("no inbound broadcast")
            .expect("broadcast channel closed")
        {
            m if m.text == "cross-peer ids" => break m,
            _ => continue,
        }
    };
    assert_eq!(
        received.sender_id.as_deref(),
        Some("id-dev"),
        "sender_id must cross the relay's explicit field copy"
    );
    assert_eq!(
        received.to_ids,
        Some(vec!["id-pm".to_string()]),
        "to_ids must cross the relay's explicit field copy"
    );
    assert_eq!(
        received.addressee_ids,
        Some(vec!["id-pm".to_string()]),
        "addressee_ids must cross the relay's explicit field copy"
    );
}

#[tokio::test]
async fn left_uses_placeholder_name_when_no_hello_received() {
    let (hub, mut rx, _dir) = make_hub();
    let (inbound_tx, inbound_rx) = mpsc::channel::<String>(16);
    let (outbound_tx, _outbound_rx) = mpsc::channel::<String>(16);

    let _peer = spawn_relay(
        hub.clone(),
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
