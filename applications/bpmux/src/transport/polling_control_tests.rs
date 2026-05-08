#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::ci_state_store::CiStateStore;
use super::polling_control::{PollAction, PollKind, PollingControl};
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

#[tokio::test]
async fn dispatch_start_ci_no_op_without_tracker() {
    let (session, _dir) = fresh_session();
    let pc = PollingControl::new(session, None, CiStateStore::new(), CancellationToken::new());
    pc.dispatch(PollAction::StartCi).await;
    assert!(!pc.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn dispatch_stop_without_start_no_op() {
    let (session, _dir) = fresh_session();
    let pc = PollingControl::new(session, None, CiStateStore::new(), CancellationToken::new());
    pc.dispatch(PollAction::Stop(PollKind::Ci)).await;
    assert!(!pc.is_running(PollKind::Ci).await);
}

#[tokio::test]
async fn parent_cancel_stops_running_poll() {
    let parent = CancellationToken::new();
    let (session, _dir) = fresh_session();
    let pc = PollingControl::new(session, None, CiStateStore::new(), parent.clone());
    pc.dispatch(PollAction::StartCi).await;
    parent.cancel();
    tokio::time::sleep(Duration::from_millis(50)).await;
    pc.dispatch(PollAction::Stop(PollKind::Ci)).await;
    assert!(!pc.is_running(PollKind::Ci).await);
}
