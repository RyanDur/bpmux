#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use crate::message::StateDeltaPayload;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::session::{self, BROADCAST_CHANNEL_CAPACITY};

#[tokio::test]
async fn stories_delta_wakes_subscribers_but_is_not_persisted_durably() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (wake_tx, _wake_rx) = tokio::sync::broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    let session =
        session::Session::new(dir.path().to_path_buf(), "test:owner".to_string(), wake_tx);
    let orch = SyncOrchestrator::new(
        Arc::new(BoardStore::new()),
        Arc::new(InMemorySourceAdapter::new()),
    );
    orch.ingest_once().await.expect("seed-empty ingest");
    let refresh: Arc<dyn RefreshDriver> = Arc::new(orch);

    let mut subscriber = session.subscribe();
    super::shared::broadcast_stories(&session, &refresh);

    let woken = subscriber
        .try_recv()
        .expect("a connected client must still receive the live stories delta");
    assert!(
        matches!(woken.state_delta, Some(StateDeltaPayload::Stories { .. })),
        "the woken delta is the stories delta"
    );

    let durable_stories_deltas = session
        .read_messages()
        .expect("read durable log")
        .iter()
        .filter(|m| matches!(m.state_delta, Some(StateDeltaPayload::Stories { .. })))
        .count();
    assert_eq!(
        durable_stories_deltas, 0,
        "stories deltas are ephemeral current-state (a reconnecting client re-hydrates fresh from the live tracker) — they must not accrete in the durable session log"
    );
}
