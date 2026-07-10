#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::time::timeout;

use super::client::ClientCtx;
use super::client_handlers::handle_send;
use crate::hub::Hub;
use crate::store::Store;

fn fresh_store() -> (Arc<Store>, Arc<Hub>) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = tokio::sync::broadcast::channel::<message::Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    std::mem::forget(dir);
    let store = Arc::new(Store::new(
        hub.clone(),
        None,
        crate::ci::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ));
    (store, hub)
}

#[tokio::test]
async fn send_from_an_identified_session_stamps_sender_id_from_routing_user_id() {
    let (store, hub) = fresh_store();
    let dispatch = store.dispatcher();
    let pty_registry = Arc::new(crate::room::LivePtyRegistry::new());
    let (pty_tx, _pty_rx) = tokio::sync::mpsc::unbounded_channel();
    let tracker: crate::transport::rest::TrackerHandle = None;
    let orchestrator: crate::sync::OrchestratorHandle = None;
    let (reply_tx, _reply_rx) = tokio::sync::mpsc::unbounded_channel();
    let routing_user_id = Some("sess-dev".to_string());
    let ctx = ClientCtx {
        hub: &hub,
        store: &store,
        dispatch: &dispatch,
        pty_registry: &pty_registry,
        pty_tx: &pty_tx,
        tracker: &tracker,
        orchestrator: &orchestrator,
        reply_tx: &reply_tx,
        routing_user_id: &routing_user_id,
    };

    let mut broadcast_rx = hub.subscribe();
    handle_send(
        &ctx,
        "dev:claude-code",
        "hello".to_string(),
        None,
        None,
        None,
    )
    .expect("handle_send");

    let broadcast = timeout(Duration::from_secs(2), broadcast_rx.recv())
        .await
        .expect("no broadcast received")
        .expect("broadcast channel closed");
    assert_eq!(
        broadcast.sender_id.as_deref(),
        Some("sess-dev"),
        "an identified session stamps sender_id from its routing_user_id, no display guessing"
    );
}
