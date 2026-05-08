#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::ci::{CiAction, CiBroadcastListener, CiSnapshot, Orchestrator, PollKind};
use crate::ci::CiState;
use crate::session::{Message, Session};

fn fresh_session() -> (Arc<Session>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    (session, dir)
}

fn snap(run_id: u64, conclusion: &'static str) -> CiSnapshot {
    CiSnapshot {
        state: match conclusion {
            "success" => CiState::Green,
            "failure" => CiState::Red,
            "in_progress" => CiState::Running,
            _ => CiState::Unknown,
        },
        run_id: Some(run_id),
        head_sha: Some("deadbeef".to_string()),
        display_title: Some("test commit".to_string()),
        conclusion,
    }
}

#[derive(Default)]
struct RecordingListener {
    received: Mutex<Vec<CiSnapshot>>,
}

impl CiBroadcastListener for RecordingListener {
    fn on_changed(&self, snap: &CiSnapshot) {
        let mut received = self.received.lock().unwrap_or_else(|e| e.into_inner());
        received.push(snap.clone());
    }
}

#[tokio::test]
async fn ci_updated_action_drives_reducer_and_fires_listener() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;

    let store = orch.ci_store();
    assert_eq!(store.last_seen(), Some((Some(42), "success")));
    assert_eq!(store.snapshot().expect("snapshot").run_id, Some(42));
    let received = recorder.received.lock().unwrap();
    assert_eq!(received.len(), 1, "listener fires on Changed outcome");
    assert_eq!(received[0].run_id, Some(42));
}

#[tokio::test]
async fn ci_updated_dedup_same_run_and_conclusion_no_listener_fire() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;

    let received = recorder.received.lock().unwrap();
    assert_eq!(received.len(), 1, "second identical CiUpdated is dedup'd");
}

#[tokio::test]
async fn ci_updated_run_change_or_conclusion_change_fires_listener() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    let recorder = Arc::new(RecordingListener::default());
    orch.add_ci_listener(recorder.clone());

    orch.dispatch(CiAction::CiUpdated(snap(42, "in_progress")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(42, "success")))
        .await;
    orch.dispatch(CiAction::CiUpdated(snap(43, "success")))
        .await;

    let received = recorder.received.lock().unwrap();
    assert_eq!(
        received.len(),
        3,
        "each transition (status flip + new run_id) fires listener"
    );
}

#[tokio::test]
async fn start_ci_polling_no_op_without_tracker() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    orch.dispatch(CiAction::StartCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn stop_ci_polling_without_start_no_op() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    orch.dispatch(CiAction::StopCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn parent_cancel_propagates_when_running() {
    let parent = CancellationToken::new();
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, parent.clone()));
    orch.dispatch(CiAction::StartCiPolling).await;
    parent.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;
    orch.dispatch(CiAction::StopCiPolling).await;
    assert!(!orch.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn rate_limit_observer_exposed_at_orchestrator_layer() {
    let (session, _dir) = fresh_session();
    let orch = Arc::new(Orchestrator::new(session, None, CancellationToken::new()));
    let snapshot = orch.rate_limit_observer().snapshot();
    assert_eq!(snapshot.retry_after_wake_epoch, 0);
    assert_eq!(snapshot.ratelimit_reset_epoch, 0);
}
