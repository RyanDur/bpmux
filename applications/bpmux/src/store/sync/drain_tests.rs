#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::ci::CiStateStore;
use crate::message::Message;
use crate::session::Session;
use crate::store::test_support::eventually;
use crate::store::{AppAction, Store, SyncAction};

fn refresh_for_tests() -> Arc<dyn crate::sync::RefreshDriver> {
    let adapter = Arc::new(
        crate::product_management::board::adapters::in_memory::InMemorySourceAdapter::new(),
    );
    Arc::new(crate::sync::SyncOrchestrator::new(adapter))
}

#[tokio::test]
async fn start_update_github_tracks_then_clears_when_drain_finds_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    let orch: Arc<Store> = Arc::new(
        Store::new(session, None, CiStateStore::new(), CancellationToken::new()).with_refresh(
            refresh_for_tests(),
            Duration::from_secs(60),
            Duration::from_secs(15),
        ),
    );

    let mutation_id = crate::sync::MutationId::default();
    orch.dispatch(AppAction::Sync(SyncAction::StartUpdateGithub {
        mutation_id,
    }))
    .await;

    let drain = orch
        .github_drain
        .as_ref()
        .expect("with_refresh installs the github drain")
        .clone();
    assert_eq!(
        drain.pending_count(),
        1,
        "StartUpdateGithub tracks the mutation token synchronously"
    );

    eventually(
        "an unknown mutation drains to NotFound, re-dispatches Stop, clears the token",
        || drain.pending_count() == 0,
    )
    .await;
}
