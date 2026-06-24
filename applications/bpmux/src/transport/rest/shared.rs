use std::sync::Arc;

use serde::Deserialize;

use crate::session;

pub type TrackerHandle = Option<Arc<crate::board::adapters::github::GitHubSourceAdapter>>;

#[derive(Deserialize)]
pub struct CommentPayload {
    pub body: String,
}

pub(super) fn err_body(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

pub(crate) fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(axum::http::header::CONTENT_TYPE, "application/json")]
}

pub(crate) fn broadcast_board_invalidate(session: &session::Session) {
    let delta = crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..crate::message::Message::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rest: board-invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn board_snapshot_message(stories: serde_json::Value) -> crate::message::Message {
    crate::message::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: crate::message::MessageType::StateDelta,
        state_delta: Some(crate::message::StateDeltaPayload::BoardSnapshot { stories }),
        ..crate::message::Message::default()
    }
}

pub(crate) fn broadcast_board_snapshot(
    session: &session::Session,
    refresh: &Arc<dyn crate::board::RefreshDriver>,
) {
    let stories = serde_json::Value::Array(super::board::ordered_board_stories(refresh));
    let delta = board_snapshot_message(stories);
    session.broadcast_ephemeral(&delta);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::sync::Arc;

    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
    use crate::message::StateDeltaPayload;
    use crate::session::{self, BROADCAST_CHANNEL_CAPACITY};

    #[tokio::test]
    async fn board_snapshot_delta_wakes_subscribers_but_is_not_persisted_durably() {
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
        super::broadcast_board_snapshot(&session, &refresh);

        let woken = subscriber
            .try_recv()
            .expect("a connected client must still receive the live board snapshot");
        assert!(
            matches!(
                woken.state_delta,
                Some(StateDeltaPayload::BoardSnapshot { .. })
            ),
            "the woken delta is the board snapshot"
        );

        let durable_board_snapshots = session
            .read_messages()
            .expect("read durable log")
            .iter()
            .filter(|m| matches!(m.state_delta, Some(StateDeltaPayload::BoardSnapshot { .. })))
            .count();
        assert_eq!(
            durable_board_snapshots, 0,
            "board snapshots are ephemeral current-state (a reconnecting client re-hydrates fresh from the live tracker) — they must not accrete in the durable session log"
        );
    }
}
